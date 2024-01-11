use std::{net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{error_handling::HandleErrorLayer, http::StatusCode, routing::post, BoxError, Router};
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
    broadcast_transaction_handler, create_transaction_handler, generate_proof_handler,
    get_balances_handler, get_transactions_handler, import_vk_handler,
};

pub mod config;

pub mod db_handler;
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
        .route("/getTransactions", post(get_transactions_handler))
        .route("/createTx", post(create_transaction_handler))
        .route("/broadcastTx", post(broadcast_transaction_handler))
        .route("/generate_proofs", post(generate_proof_handler))
        .with_state(shared_state)
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|_: BoxError| async {
                    StatusCode::REQUEST_TIMEOUT
                }))
                .layer(TimeoutLayer::new(Duration::from_secs(30)))
                .layer(CorsLayer::new().allow_methods(Any).allow_origin(Any)),
        );

    let listener = TcpListener::bind(&listen).await?;
    axum::serve(listener, router).await?;
    info!("Server listening on {}", listen);
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
    use bellperson::groth16::Proof;
    use blstrs::Bls12;
    use ff::{Field, PrimeField, PrimeFieldBits};
    use group::{Curve, Group};
    use ironfish_zkp::constants::PUBLIC_KEY_GENERATOR;
    use ironfish_zkp::proofs::{MintAsset, Output};
    use ironfish_zkp::util::{asset_hash_to_point, commitment_full_point};
    use ironfish_zkp::{primitives::ValueCommitment, proofs::Spend};
    use jubjub::ExtendedPoint;
    use rand::Rng;
    use rand::{rngs::StdRng, RngCore, SeedableRng};
    use zcash_primitives::constants::VALUE_COMMITMENT_VALUE_GENERATOR;
    use zcash_primitives::sapling::{pedersen_hash, Note, ProofGenerationKey, Rseed};

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

        let sender_address = payment_address;

        let rk = jubjub::ExtendedPoint::from(viewing_key.rk(ar)).to_affine();

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
        let incoming_view_key = proof_generation_key.to_viewing_key();
        let public_address = *PUBLIC_KEY_GENERATOR * incoming_view_key.ivk().0;
        let public_address_point = ExtendedPoint::from(public_address).to_affine();

        let public_key_randomness = jubjub::Fr::random(&mut rng);
        let randomized_public_key =
            ExtendedPoint::from(incoming_view_key.rk(public_key_randomness)).to_affine();

        let public_inputs = vec![
            randomized_public_key.get_u(),
            randomized_public_key.get_v(),
            public_address_point.get_u(),
            public_address_point.get_v(),
        ];

        // Mint proof
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
}
