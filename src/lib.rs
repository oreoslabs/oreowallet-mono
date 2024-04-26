use std::{cmp, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{get, post},
    BoxError, Router,
};
use chrono::Utc;
use constants::{REORG_DEPTH, SECONDARY_BATCH};
use db_handler::{Account, DBHandler, PgHandler};
use manager::{codec::DRequest, Manager, TaskInfo};
use rpc_handler::{abi::RpcTransaction, RpcHandler};
use tokio::{net::TcpListener, sync::oneshot, time::sleep};
use tower::{timeout::TimeoutLayer, ServiceBuilder};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::{
    manager::ServerMessage,
    web_handlers::{
        account_status_handler, account_transaction_handler, broadcast_transaction_handler,
        create_transaction_handler, generate_proof_handler, get_balances_handler, get_ores_handler,
        get_transactions_handler, import_vk_handler, latest_block_handler, remove_account_handler,
    },
};

pub mod config;
pub mod constants;
pub mod db_handler;
pub mod dworkers;
pub mod error;
pub mod manager;
pub mod orescriptions;
pub mod rpc_handler;
pub mod web_handlers;

#[derive(Debug, Clone)]
pub struct SharedState<T: DBHandler> {
    pub db_handler: T,
    pub rpc_handler: RpcHandler,
}

impl<T> SharedState<T>
where
    T: DBHandler,
{
    pub fn new(db_handler: T, endpoint: &str) -> Self {
        Self {
            db_handler: db_handler,
            rpc_handler: RpcHandler::new(endpoint.into()),
        }
    }
}

pub async fn scheduling_tasks(
    manager: Arc<Manager>,
    account: &Account,
    block_hash: &str,
    block_sequence: i64,
    transactions: &Vec<RpcTransaction>,
    status: u8,
) {
    info!(
        "scheduling task for account {} at sequence {}",
        account.name, block_sequence
    );
    let task = DRequest::from_transactions(account, transactions);
    let task_id = task.id.clone();
    let _ = manager.task_mapping.write().await.insert(
        task_id,
        TaskInfo {
            timestampt: Utc::now().timestamp(),
            hash: block_hash.to_string(),
            sequence: block_sequence,
            status,
        },
    );
    for (k, worker) in manager.workers.read().await.iter() {
        if worker.status == 1 {
            if let Err(e) = worker
                .router
                .send(ServerMessage {
                    name: Some(k.to_string()),
                    request: task.clone(),
                })
                .await
            {
                error!("failed to send task to manager {}", e);
            } else {
                return;
            }
        }
    }
    let _ = manager.task_queue.write().await.push(task);
}

pub async fn run_server(
    listen: SocketAddr,
    rpc_server: String,
    db_handler: PgHandler,
    dlistener: SocketAddr,
    decryption: bool,
) -> Result<()> {
    let shared_resource = Arc::new(SharedState::new(db_handler, &rpc_server));
    if !decryption {
        let (router, handler) = oneshot::channel();
        let _ = tokio::spawn(async move {
            let _ = router.send(());
            let _ = start_rest_service(listen, shared_resource.clone()).await;
        });
        let _ = handler.await;
        std::future::pending::<()>().await;
        return Ok(());
    }

    let manager = Manager::new(shared_resource.clone());
    let listener = TcpListener::bind(&dlistener).await.unwrap();

    // dworker connection handler
    let (router, handler) = oneshot::channel();
    let dworker_manager = manager.clone();
    let dworker_handler = tokio::spawn(async move {
        let _ = router.send(());
        loop {
            match listener.accept().await {
                Ok((stream, ip)) => {
                    info!("new connection from {}", ip);
                    if let Err(e) = Manager::handle_stream(stream, dworker_manager.clone()).await {
                        error!("failed to handle stream, {e}");
                    }
                }
                Err(e) => error!("failed to accept connection, {:?}", e),
            }
        }
    });
    let _ = handler.await;

    // primary loop
    // scheduling decryption task for accounts whos head within [latest.block.sequence - REORG_DEPTH, latest.block.sequence]
    // chain reorg should be handled during primary scanning carefully
    let (router, handler) = oneshot::channel();
    let primary_scheduling_manager = manager.clone();
    let primary_handler = tokio::spawn(async move {
        let _ = router.send(());

        // warmup, wait for dworkers
        {
            sleep(Duration::from_secs(30)).await;
        }
        loop {
            let chain_head = primary_scheduling_manager
                .shared
                .rpc_handler
                .get_latest_block()
                .await
                .unwrap()
                .data
                .current_block_identifier;
            let chain_height = chain_head.index.parse::<i64>().unwrap();
            let start_seq = chain_height - REORG_DEPTH;
            let end_seq = chain_height + 1;
            if let Ok(accounts) = primary_scheduling_manager
                .shared
                .db_handler
                .get_accounts_with_head(start_seq)
                .await
            {
                if accounts.len() == 0 {
                    info!("empty accounts to handle in primary_scheduling");
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                for seq in start_seq..end_seq {
                    let mut should_break = false;
                    if let Ok(response) = primary_scheduling_manager
                        .shared
                        .rpc_handler
                        .get_block(seq)
                        .await
                    {
                        let current_block_hash = response.data.block.hash;
                        let transactions = response.data.block.transactions;
                        for acc in accounts.iter() {
                            // Update account head/hash, createHead/hash for new created account
                            if acc.head == seq
                                && acc.create_head.is_some()
                                && acc.create_head.unwrap() == seq
                            {
                                let _ = primary_scheduling_manager
                                    .shared
                                    .db_handler
                                    .update_account_head(
                                        acc.address.clone(),
                                        seq,
                                        current_block_hash.clone(),
                                    )
                                    .await;
                                let _ = primary_scheduling_manager
                                    .shared
                                    .db_handler
                                    .update_account_createdhead(
                                        acc.address.clone(),
                                        seq,
                                        current_block_hash.clone(),
                                    )
                                    .await;
                                continue;
                            }

                            // rollback to right onchain block for accounts on forked chain
                            if acc.head == seq && acc.hash != current_block_hash.clone() {
                                let mut sequence = seq;
                                let mut hash = current_block_hash.clone();
                                loop {
                                    if acc.create_head.is_some()
                                        && acc.create_head.unwrap() == sequence
                                    {
                                        let _ = primary_scheduling_manager
                                            .shared
                                            .db_handler
                                            .update_account_createdhead(
                                                acc.address.clone(),
                                                sequence,
                                                hash.clone(),
                                            )
                                            .await;
                                        break;
                                    }
                                    if let Ok(unstable) = primary_scheduling_manager
                                        .shared
                                        .db_handler
                                        .get_primary_account(acc.address.to_string(), sequence)
                                        .await
                                    {
                                        if unstable.hash != hash.clone() {
                                            let _ = primary_scheduling_manager
                                                .shared
                                                .db_handler
                                                .del_primary_account(acc.address.clone(), sequence)
                                                .await;
                                            sequence -= 1;
                                            hash = primary_scheduling_manager
                                                .shared
                                                .rpc_handler
                                                .get_block(sequence)
                                                .await
                                                .unwrap()
                                                .data
                                                .block
                                                .hash;
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                let _ = primary_scheduling_manager
                                    .shared
                                    .db_handler
                                    .update_account_head(acc.address.clone(), sequence, hash)
                                    .await;
                                should_break = true;
                            } else {
                                let _ = scheduling_tasks(
                                    primary_scheduling_manager.clone(),
                                    &acc,
                                    &current_block_hash,
                                    seq,
                                    &transactions,
                                    0,
                                )
                                .await;
                            }
                        }
                        if should_break {
                            break;
                        }
                    }
                }
            }
            sleep(Duration::from_secs(10)).await;
        }
    });
    let _ = handler.await;

    // secondary loop
    // scheduling decryption task for accounts whos head jump out [latest.block.sequence - REORG_DEPTH, latest.block.sequence]
    let (router, handler) = oneshot::channel();
    let secondary_scheduling_manager = manager.clone();
    let secondary_handler = tokio::spawn(async move {
        let _ = router.send(());

        // warmup, wait for dworkers
        {
            sleep(Duration::from_secs(30)).await;
        }
        loop {
            if let Ok(accounts) = secondary_scheduling_manager
                .shared
                .db_handler
                .get_oldest_accounts()
                .await
            {
                if accounts.len() == 0 {
                    info!("empty accounts to handle in secondary_scheduling");
                    sleep(Duration::from_secs(3)).await;
                    continue;
                }
                let chain_head = secondary_scheduling_manager
                    .shared
                    .rpc_handler
                    .get_latest_block()
                    .await
                    .unwrap()
                    .data
                    .current_block_identifier;
                let account_head = accounts[0].head;
                if account_head < chain_head.index.parse::<i64>().unwrap() - REORG_DEPTH {
                    let start_seq = account_head;
                    let end_seq = cmp::min(
                        account_head + SECONDARY_BATCH,
                        chain_head.index.parse::<i64>().unwrap() - REORG_DEPTH,
                    );
                    info!("start scanning from {} to {}", start_seq, end_seq);
                    for seq in start_seq..end_seq {
                        let mut should_break = false;
                        if let Ok(response) = secondary_scheduling_manager
                            .shared
                            .rpc_handler
                            .get_block(seq)
                            .await
                        {
                            let previous_block_hash = response.data.block.previous_block_hash;
                            let current_block_hash = response.data.block.hash;
                            let transactions = response.data.block.transactions;
                            for acc in accounts.iter() {
                                // this should never happen in secondary_scheduling
                                if acc.head == seq && acc.hash != current_block_hash.clone() {
                                    warn!(
                                        "block hash doesn't match, unexpected chain reorg happens at sequence {}", seq
                                    );
                                    warn!("should never happen in secondary_scheduling");
                                    let _ = secondary_scheduling_manager
                                        .shared
                                        .db_handler
                                        .update_account_head(
                                            acc.address.clone(),
                                            seq - 1,
                                            previous_block_hash.clone(),
                                        )
                                        .await;
                                    should_break = true;
                                } else {
                                    let _ = scheduling_tasks(
                                        secondary_scheduling_manager.clone(),
                                        &acc,
                                        &current_block_hash,
                                        seq,
                                        &transactions,
                                        1,
                                    )
                                    .await;
                                }
                            }
                            if should_break {
                                break;
                            }
                        }
                    }
                }
            }
            sleep(Duration::from_secs(10)).await;
        }
    });
    let _ = handler.await;

    // manager status updater
    let status_manager = manager.clone();
    let (router, handler) = oneshot::channel();
    let status_update_handler = tokio::spawn(async move {
        let _ = router.send(());
        loop {
            {
                let workers = status_manager.workers.read().await;
                let workers: Vec<&String> = workers.keys().collect();
                let pending_taskes = status_manager.task_queue.read().await.len();
                info!("online workers: {}, {:?}", workers.len(), workers);
                info!("pending taskes in queue: {}", pending_taskes);
            }
            sleep(Duration::from_secs(10)).await;
        }
    });
    let _ = handler.await;

    // restful api handler
    let (router, handler) = oneshot::channel();
    let rest_handler = tokio::spawn(async move {
        let _ = router.send(());
        let _ = start_rest_service(listen, shared_resource.clone()).await;
    });
    let _ = handler.await;

    let _ = tokio::join!(
        dworker_handler,
        rest_handler,
        status_update_handler,
        primary_handler,
        secondary_handler
    );
    std::future::pending::<()>().await;
    Ok(())
}

pub async fn start_rest_service(
    listen: SocketAddr,
    shared_state: Arc<SharedState<PgHandler>>,
) -> Result<()> {
    let router = Router::new()
        .route("/import", post(import_vk_handler))
        .route("/remove", post(remove_account_handler))
        .route("/getBalances", post(get_balances_handler))
        .route("/getTransaction", post(account_transaction_handler))
        .route("/getTransactions", post(get_transactions_handler))
        .route("/createTx", post(create_transaction_handler))
        .route("/broadcastTx", post(broadcast_transaction_handler))
        .route("/accountStatus", post(account_status_handler))
        .route("/latestBlock", get(latest_block_handler))
        .route("/ores", post(get_ores_handler))
        .with_state(shared_state.clone())
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

pub async fn run_prover(listen: SocketAddr) -> Result<()> {
    let router = Router::new()
        .route("/generateProofs", post(generate_proof_handler))
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
    info!("Prover listening on {}", listen);
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
}
