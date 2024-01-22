use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
};
use db_handler::{DBHandler, RedisClient};
use rpc_handler::RpcHandler;
use tokio::{
    net::TcpListener,
    sync::{oneshot, Mutex},
};
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::web_handlers::{
    account_status_handler, account_transaction_handler, broadcast_transaction_handler,
    create_transaction_handler, generate_proof_handler, get_balances_handler, get_ores_handler,
    get_transactions_handler, import_vk_handler, latest_block_handler,
};

pub mod constants;
pub mod db_handler;
pub mod error;
pub mod orescriptions;
pub mod rpc_handler;
pub mod web_handlers;

#[derive(Debug, Clone)]
pub struct SharedState<T: DBHandler> {
    pub db_handler: Arc<Mutex<T>>,
    pub rpc_handler: RpcHandler,
}

impl<T> SharedState<T>
where
    T: DBHandler,
{
    pub fn new(db_handler: T, endpoint: &str) -> Self {
        Self {
            db_handler: Arc::new(Mutex::new(db_handler)),
            rpc_handler: RpcHandler::new(endpoint.into()),
        }
    }
}

pub async fn run_server(listen: SocketAddr, rpc_server: String, redis: String) -> Result<()> {
    let db_handler = RedisClient::init(&redis);
    let shared_state = SharedState::new(db_handler, &rpc_server);
    let router = Router::new()
        .route("/import", post(import_vk_handler))
        .route("/getBalances", post(get_balances_handler))
        .route("/getTransaction", post(account_transaction_handler))
        .route("/getTransactions", post(get_transactions_handler))
        .route("/createTx", post(create_transaction_handler))
        .route("/broadcastTx", post(broadcast_transaction_handler))
        .route("/generateProofs", post(generate_proof_handler))
        .route("/accountStatus", post(account_status_handler))
        .route("/latestBlock", get(latest_block_handler))
        .route("/ores", post(get_ores_handler))
        .with_state(shared_state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(30))),
        )
        .layer(
            CorsLayer::new()
                .allow_methods(Any)
                .allow_origin(Any)
                .allow_headers(Any),
        );

    let listener = TcpListener::bind(&listen).await?;
    info!("Server listening on {}", listen);
    axum::serve(listener, router).await?;
    Ok(())
}

pub fn initialize_logger(verbosity: u8) {
    match verbosity {
        0 => std::env::set_var("RUST_LOG", "info"),
        1 => std::env::set_var("RUST_LOG", "debug"),
        2 | 3 | 4 => std::env::set_var("RUST_LOG", "trace"),
        _ => std::env::set_var("RUST_LOG", "info"),
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("ironfish-server=info".parse().unwrap()),
        )
        .init();
}

pub async fn handle_signals() -> anyhow::Result<()> {
    let (router, handler) = oneshot::channel();
    tokio::spawn(async move {
        let _ = router.send(());
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                info!("Shutdowning...");
                tokio::time::sleep(Duration::from_millis(5000)).await;
                info!("Goodbye");
                std::process::exit(0);
            }
            Err(error) => warn!("tokio::signal::ctrl_c encountered an error: {}", error),
        }
    });
    let _ = handler.await;
    info!("Signal handler installed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::response::IntoResponse;
    use bellperson::groth16::Proof;
    use blstrs::Bls12;
    use ff::{Field, PrimeField, PrimeFieldBits};
    use group::{Curve, Group};
    use ironfish_zkp::constants::PUBLIC_KEY_GENERATOR;
    use ironfish_zkp::proofs::{MintAsset, Output};
    use ironfish_zkp::util::{asset_hash_to_point, commitment_full_point};
    use ironfish_zkp::{primitives::ValueCommitment, proofs::Spend};
    use rand::Rng;
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use zcash_primitives::constants::VALUE_COMMITMENT_VALUE_GENERATOR;
    use zcash_primitives::sapling::{pedersen_hash, Note, ProofGenerationKey, Rseed};

    use crate::error::OreoError;
    use crate::web_handlers::abi::{GenerateProofRep, GenerateProofReq};

    fn build_spend() -> Spend {
        let mut rng = StdRng::seed_from_u64(0);
        let tree_depth = 32;

        let value_commitment = ValueCommitment {
            value: rng.next_u64(),
            randomness: jubjub::Fr::random(&mut rng),
            asset_generator: (*VALUE_COMMITMENT_VALUE_GENERATOR).into(),
        };

        let proof_generation_key = ProofGenerationKey {
            ak: jubjub::SubgroupPoint::random(&mut rng),
            nsk: jubjub::Fr::random(&mut rng),
        };

        let viewing_key = proof_generation_key.to_viewing_key();

        let payment_address = *PUBLIC_KEY_GENERATOR * viewing_key.ivk().0;
        let commitment_randomness = jubjub::Fr::random(&mut rng);
        let auth_path =
            vec![Some((blstrs::Scalar::random(&mut rng), rng.next_u32() % 2 != 0)); tree_depth];
        let ar = jubjub::Fr::random(&mut rng);

        let note = Note {
            value: value_commitment.value,
            g_d: *PUBLIC_KEY_GENERATOR,
            pk_d: payment_address,
            rseed: Rseed::BeforeZip212(commitment_randomness),
        };

        let commitment = commitment_full_point(
            value_commitment.asset_generator,
            value_commitment.value,
            payment_address,
            note.rcm(),
            payment_address,
        );
        let cmu = jubjub::ExtendedPoint::from(commitment).to_affine().get_u();

        let mut cur = cmu;

        for (i, val) in auth_path.clone().into_iter().enumerate() {
            let (uncle, b) = val.unwrap();

            let mut lhs = cur;
            let mut rhs = uncle;

            if b {
                ::std::mem::swap(&mut lhs, &mut rhs);
            }

            let lhs = lhs.to_le_bits();
            let rhs = rhs.to_le_bits();

            cur = jubjub::ExtendedPoint::from(pedersen_hash::pedersen_hash(
                pedersen_hash::Personalization::MerkleTree(i),
                lhs.iter()
                    .by_vals()
                    .take(blstrs::Scalar::NUM_BITS as usize)
                    .chain(rhs.iter().by_vals().take(blstrs::Scalar::NUM_BITS as usize)),
            ))
            .to_affine()
            .get_u();
        }

        Spend {
            value_commitment: Some(value_commitment.clone()),
            proof_generation_key: Some(proof_generation_key.clone()),
            payment_address: Some(payment_address),
            commitment_randomness: Some(commitment_randomness),
            ar: Some(ar),
            auth_path: auth_path.clone(),
            anchor: Some(cur),
            sender_address: Some(payment_address),
        }
    }

    fn build_output() -> Output {
        let mut rng = StdRng::seed_from_u64(0);
        let mut asset_id = [0u8; 32];
        let asset_generator = loop {
            rng.fill(&mut asset_id[..]);

            if let Some(point) = asset_hash_to_point(&asset_id) {
                break point;
            }
        };
        let value_commitment_randomness = jubjub::Fr::random(&mut rng);
        let note_commitment_randomness = jubjub::Fr::random(&mut rng);
        let value_commitment = ValueCommitment {
            value: rng.next_u64(),
            randomness: value_commitment_randomness,
            asset_generator,
        };

        let nsk = jubjub::Fr::random(&mut rng);
        let ak = jubjub::SubgroupPoint::random(&mut rng);
        let esk = jubjub::Fr::random(&mut rng);
        let ar = jubjub::Fr::random(&mut rng);

        let proof_generation_key = ProofGenerationKey { ak, nsk };

        let viewing_key = proof_generation_key.to_viewing_key();

        let payment_address = *PUBLIC_KEY_GENERATOR * viewing_key.ivk().0;

        Output {
            value_commitment: Some(value_commitment.clone()),
            payment_address: Some(payment_address),
            commitment_randomness: Some(note_commitment_randomness),
            esk: Some(esk),
            asset_id,
            proof_generation_key: Some(proof_generation_key.clone()),
            ar: Some(ar),
        }
    }

    fn build_mint_asset() -> MintAsset {
        let mut rng = StdRng::seed_from_u64(0);
        let proof_generation_key = ProofGenerationKey {
            ak: jubjub::SubgroupPoint::random(&mut rng),
            nsk: jubjub::Fr::random(&mut rng),
        };

        let public_key_randomness = jubjub::Fr::random(&mut rng);

        MintAsset {
            proof_generation_key: Some(proof_generation_key),
            public_key_randomness: Some(public_key_randomness),
        }
    }

    #[tokio::test]
    async fn generate_proofs_works() {
        let client = reqwest::Client::new();
        let spend = build_spend();
        let output = build_output();
        let mint_asset = build_mint_asset();
        let mut spend_bytes = vec![];
        spend.write(&mut spend_bytes).unwrap();
        let mut output_bytes = vec![];
        output.write(&mut output_bytes).unwrap();
        let mut mint_asset_bytes = vec![];
        mint_asset.write(&mut mint_asset_bytes).unwrap();
        let body = GenerateProofReq {
            spend_circuits: vec![spend_bytes],
            output_circuits: vec![output_bytes],
            mint_asset_circuits: vec![mint_asset_bytes],
        };
        let response = client
            .post("http://127.0.0.1:10001/generate_proofs")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .expect("failed to generate proofs");
        assert!(response.status().is_success());
        let rep: GenerateProofRep = response.json().await.unwrap();
        if let Some(proof) = rep.spend_proofs.first() {
            let proof: Proof<Bls12> = Proof::read(&proof[..]).unwrap();
            println!("response first spend proof {:?}", proof);
        }
        if let Some(proof) = rep.output_proofs.first() {
            let proof: Proof<Bls12> = Proof::read(&proof[..]).unwrap();
            println!("response first output proof {:?}", proof);
        }
        if let Some(proof) = rep.mint_asset_proofs.first() {
            let proof: Proof<Bls12> = Proof::read(&proof[..]).unwrap();
            println!("response first mint asset proof {:?}", proof);
        }
    }

    #[tokio::test]
    async fn generate_proofs_failed() {
        let client = reqwest::Client::new();
        let spend = build_spend();
        let spend2 = build_spend();
        let output = build_output();
        let mint_asset = build_mint_asset();
        let mut spend_bytes = vec![];
        spend.write(&mut spend_bytes).unwrap();
        let mut spend2_bytes = vec![];
        spend2.write(&mut spend2_bytes).unwrap();
        let mut output_bytes = vec![];
        output.write(&mut output_bytes).unwrap();
        let mut mint_asset_bytes = vec![];
        mint_asset.write(&mut mint_asset_bytes).unwrap();

        // make spend2 circuit fail to generate a proof
        spend2_bytes.truncate(100);

        let body = GenerateProofReq {
            spend_circuits: vec![spend_bytes, spend2_bytes],
            output_circuits: vec![output_bytes],
            mint_asset_circuits: vec![mint_asset_bytes],
        };
        let response = client
            .post("http://127.0.0.1:10001/generate_proofs")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .expect("failed to generate proofs");

        let status1 = response.status();
        assert!(status1.is_success());

        let error = response.json::<OreoError>().await.unwrap();
        let error_msg = error.to_string();
        assert_eq!(
            error_msg,
            OreoError::GenerateSpendProofFailed(1).to_string()
        );

        let response = error.into_response();
        let status = response.status();
        assert_eq!(status.as_u16(), 900);
    }

    // account used for tests
    //     Mnemonic  eight fog reward cat spoon lawsuit mention mean number wine female asthma adapt flush salad slam rib desert goddess flame code pass turn route
    //  Spending Key  46eb4ae291ed28fc62c44e977f7153870030b3af9658b8e77590ac22d1417ab5
    //      View Key  4ae4eb9606ba57b3b17a444100a9ac6453cd67e6fe4c860e63a2e18b1200978ab5ecce68e8639d5016cbe73b0ea9a3c8e906fc881af2e9ccfa7a7b63fb73d555
    //   Incoming View Key  4a08bec0ec5a471352f340d737e4b3baec2aec8d0a2e12201d92d8ad71aadd07
    //   Outgoing View Key  cee4ff41d7d8da5eedc6493134981eaad7b26a8b0291a4eac9ba95090fa47bf7
    //       Address  d63ba13d7c35caf942c64d5139b948b885ec931977a3f248c13e7f3c1bd0aa64

    const VK: &str = "4ae4eb9606ba57b3b17a444100a9ac6453cd67e6fe4c860e63a2e18b1200978ab5ecce68e8639d5016cbe73b0ea9a3c8e906fc881af2e9ccfa7a7b63fb73d555";
    const IN_VK: &str = "4a08bec0ec5a471352f340d737e4b3baec2aec8d0a2e12201d92d8ad71aadd07";
    const OUT_VK: &str = "cee4ff41d7d8da5eedc6493134981eaad7b26a8b0291a4eac9ba95090fa47bf7";
    const ADDRESS: &str = "d63ba13d7c35caf942c64d5139b948b885ec931977a3f248c13e7f3c1bd0aa64";
}
