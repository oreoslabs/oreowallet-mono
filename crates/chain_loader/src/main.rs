use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use anyhow::Result;
use db_handler::{load_db, DBTransaction, InnerBlock, Json};
use networking::{rpc_abi::RpcBlock, rpc_handler::RpcHandler};
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use tokio::{sync::oneshot, time::sleep};
use tracing::{error, info};
use utils::{
    blocks_range, initialize_logger, initialize_logger_filter, ChainLoader, EnvFilter, Parser,
};

async fn start<N: Network>(chain_loader: ChainLoader, shut_down: Arc<AtomicBool>) -> Result<()> {
    let rpc_handler = RpcHandler::new(chain_loader.node.clone());
    let genesis_block = rpc_handler
        .get_latest_block()
        .unwrap()
        .data
        .genesis_block_identifier;
    if genesis_block.hash.to_lowercase() != N::GENESIS_BLOCK_HASH.to_lowercase() {
        panic!("Genesis block doesn't match");
    }

    let db_handler = { load_db(chain_loader.dbconfig.clone()).unwrap() };

    for group in blocks_range(1..N::LOCAL_BLOCKS_CHECKPOINT + 1, N::PRIMARY_BATCH) {
        if shut_down.load(Ordering::Relaxed) {
            info!("Chainloader should exit now");
            break;
        }
        if db_handler
            .get_blocks(group.start as i64, group.end as i64)
            .await
            .is_ok()
        {
            continue;
        }
        let results = {
            loop {
                match rpc_handler.get_blocks(group.start, group.end) {
                    Ok(res) => break res,
                    Err(e) => {
                        error!("Failed to get rpc blocks {}", e);
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
        };
        let blocks: Vec<RpcBlock> = results
            .data
            .blocks
            .into_iter()
            .map(|item| item.block)
            .collect();
        let inner_blocks = blocks
            .into_iter()
            .map(|rpc| InnerBlock {
                hash: rpc.hash.clone(),
                sequence: rpc.sequence as i64,
                transactions: Json(
                    rpc.transactions
                        .into_iter()
                        .map(|tx| DBTransaction {
                            hash: tx.hash,
                            serialized_notes: tx.notes.into_iter().map(|n| n.serialized).collect(),
                        })
                        .collect(),
                ),
            })
            .collect();
        if group.end % 1000 == 0 {
            info!(
                "save blocks from {} to {} in local db",
                group.start, group.end
            );
        }
        let _ = db_handler.save_blocks(inner_blocks).await;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let chain_loader = ChainLoader::parse();
    initialize_logger(chain_loader.verbosity);
    initialize_logger_filter(EnvFilter::from_default_env());
    let shut_down = Arc::new(AtomicBool::new(false));
    handle_signals(shut_down.clone()).await;
    match chain_loader.network {
        Mainnet::ID => start::<Mainnet>(chain_loader, shut_down).await?,
        Testnet::ID => start::<Testnet>(chain_loader, shut_down).await?,
        _ => panic!("Invalid network used"),
    }
    Ok(())
}

async fn handle_signals(shut_down: Arc<AtomicBool>) {
    let (router, handler) = oneshot::channel();
    tokio::spawn(async move {
        let _ = router.send(());
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                shut_down.store(true, Ordering::SeqCst);
                info!("Shutdown signal received, exit after 3 seconds");
                sleep(Duration::from_secs(3)).await;
                std::process::exit(0);
            }
            Err(error) => error!("tokio::signal::ctrl_c encountered an error: {}", error),
        }
    });
    let _ = handler.await;
    info!("Shutdown handler installed");
}
