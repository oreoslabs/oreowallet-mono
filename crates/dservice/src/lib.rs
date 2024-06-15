use std::{
    cmp::{self, Reverse},
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use constants::{
    LOCAL_BLOCKS_CHECKPOINT, MAINNET_GENESIS_HASH, MAINNET_GENESIS_SEQUENCE, PRIMARY_BATCH,
    REORG_DEPTH, RESCHEDULING_DURATION,
};
use db_handler::{Account, DBHandler, InnerBlock, PgHandler};
use manager::{AccountInfo, Manager, ServerMessage, SharedState, TaskInfo};
use networking::{
    rpc_abi::{BlockInfo, RpcGetAccountStatusRequest},
    socket_message::codec::DRequest,
};
use tokio::{net::TcpListener, sync::oneshot, time::sleep};
use tracing::{debug, error, info};
use utils::blocks_range;

pub mod manager;
pub mod router;

pub async fn scheduling_tasks(
    scheduler: Arc<Manager>,
    accounts: &Vec<Account>,
    blocks: Vec<InnerBlock>,
) -> anyhow::Result<()> {
    for account in accounts.iter() {
        if let Some(account_info) = scheduler
            .account_mappling
            .read()
            .await
            .get(&account.address)
        {
            info!(
                "start scanning {} blocks for account {:?}",
                blocks.len(),
                account.address.clone()
            );
            for block in blocks.iter() {
                if (block.sequence as u64) < account_info.start_block.sequence
                    || (block.sequence as u64) > account_info.end_block.sequence
                {
                    debug!("skip height {:?}", block.sequence);
                    continue;
                }
                let task = DRequest::from_transactions(account, block.transactions.clone());
                let task_id = task.id.clone();
                let _ = scheduler.task_mapping.write().await.insert(
                    task_id,
                    TaskInfo {
                        since: Instant::now(),
                        sequence: block.sequence,
                        hash: block.hash.clone(),
                        address: account.address.clone(),
                    },
                );
                let mut task_sent = false;
                for (k, worker) in scheduler.workers.read().await.iter() {
                    if worker.status == 1 {
                        match worker
                            .router
                            .send(ServerMessage {
                                name: Some(k.to_string()),
                                request: task.clone(),
                            })
                            .await
                        {
                            Ok(_) => {
                                task_sent = true;
                                break;
                            }
                            Err(e) => {
                                error!("failed to send message to worker, {:?}", e);
                            }
                        }
                    }
                }
                if task_sent {
                    continue;
                }
                let _ = scheduler
                    .task_queue
                    .write()
                    .await
                    .push(task, Reverse(block.sequence));
            }
        }
    }
    Ok(())
}

pub async fn run_dserver(
    dlisten: SocketAddr,
    rpc_server: String,
    db_handler: PgHandler,
    server: String,
) -> anyhow::Result<()> {
    let shared_resource = Arc::new(SharedState::new(db_handler, &rpc_server));
    let manager = Manager::new(shared_resource, server);
    let listener = TcpListener::bind(&dlisten).await.unwrap();

    // dworker handler
    let (router, handler) = oneshot::channel();
    let dworker_manager = manager.clone();
    let dworker_handler = tokio::spawn(async move {
        let _ = router.send(());
        loop {
            match listener.accept().await {
                Ok((stream, ip)) => {
                    debug!("new connection from {}", ip);
                    if let Err(e) = Manager::handle_stream(stream, dworker_manager.clone()).await {
                        error!("failed to handle stream, {e}");
                    }
                }
                Err(e) => error!("failed to accept connection, {:?}", e),
            }
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
                info!("online workers: {}, {:?}", workers.len(), workers);
                info!(
                    "pending taskes in queue: {}",
                    status_manager.task_queue.read().await.len()
                );
            }
            sleep(Duration::from_secs(60)).await;
        }
    });
    let _ = handler.await;

    // primary task scheduling
    let schduler = manager.clone();
    let (router, handler) = oneshot::channel();
    let scheduling_handler = tokio::spawn(async move {
        let _ = router.send(());
        loop {
            if let Ok(accounts) = schduler.shared.db_handler.get_many_need_scan().await {
                if !accounts.is_empty() {
                    let mut accounts_should_scan = vec![];
                    let mut scan_start = u64::MAX;
                    let latest = schduler
                        .shared
                        .rpc_handler
                        .get_latest_block()
                        .unwrap()
                        .data
                        .current_block_identifier;
                    let scan_end = schduler
                        .shared
                        .rpc_handler
                        .get_block(latest.index.parse::<i64>().unwrap() - REORG_DEPTH)
                        .unwrap()
                        .data
                        .block;
                    let scan_end = BlockInfo {
                        sequence: scan_end.sequence as u64,
                        hash: scan_end.hash,
                    };
                    for account in accounts {
                        if schduler
                            .account_mappling
                            .read()
                            .await
                            .get(&account.address)
                            .is_some()
                        {
                            continue;
                        }
                        if let Ok(status) = schduler.shared.rpc_handler.get_account_status(
                            RpcGetAccountStatusRequest {
                                account: account.name.clone(),
                            },
                        ) {
                            match status.data.account.head {
                                Some(head) => {
                                    let _ = schduler.account_mappling.write().await.insert(
                                        account.address.clone(),
                                        AccountInfo::new(head.clone(), scan_end.clone()),
                                    );
                                    scan_start = cmp::min(scan_start, head.sequence);
                                    accounts_should_scan.push(account);
                                }
                                None => {
                                    let _ = schduler.account_mappling.write().await.insert(
                                        account.address.clone(),
                                        AccountInfo::new(
                                            BlockInfo {
                                                hash: MAINNET_GENESIS_HASH.to_string(),
                                                sequence: MAINNET_GENESIS_SEQUENCE as u64,
                                            },
                                            scan_end.clone(),
                                        ),
                                    );
                                    scan_start = cmp::min(scan_start, 1);
                                    accounts_should_scan.push(account);
                                }
                            }
                        }
                    }
                    info!("accounts to scanning, {:?}", accounts_should_scan);
                    let blocks_to_scan =
                        blocks_range(scan_start..scan_end.sequence + 1, PRIMARY_BATCH);
                    for group in blocks_to_scan {
                        let blocks = match group.end <= LOCAL_BLOCKS_CHECKPOINT {
                            true => schduler
                                .shared
                                .db_handler
                                .get_blocks(group.start as i64, group.end as i64)
                                .await
                                .unwrap(),
                            false => {
                                let items = schduler
                                    .shared
                                    .rpc_handler
                                    .get_blocks(group.start, group.end)
                                    .unwrap()
                                    .data
                                    .blocks;
                                items
                                    .into_iter()
                                    .map(|item| item.block.to_inner())
                                    .collect()
                            }
                        };

                        let _ = scheduling_tasks(schduler.clone(), &accounts_should_scan, blocks)
                            .await
                            .unwrap();
                    }
                }
            }
            sleep(RESCHEDULING_DURATION).await;
        }
    });
    let _ = handler.await;

    // secondary task scheduling
    let secondary = manager.clone();
    let (router, handler) = oneshot::channel();
    let secondary_scheduling_handler = tokio::spawn(async move {
        let _ = router.send(());
        loop {
            for key in secondary
                .task_mapping
                .read()
                .await
                .iter()
                .filter(|(_k, v)| v.since.elapsed().as_secs() >= 600)
                .map(|(k, _)| k.to_string())
            {
                info!("rescheduling task: {:?}", key);
                match secondary.task_mapping.write().await.remove(&key) {
                    Some(task_info) => {
                        let address = task_info.address.to_string();
                        let sequence = task_info.sequence;
                        if let Ok(account) = secondary
                            .shared
                            .db_handler
                            .get_account(address.clone())
                            .await
                        {
                            if let Ok(block) = secondary.shared.rpc_handler.get_block(sequence) {
                                let block = block.data.block.to_inner();
                                let _ = scheduling_tasks(
                                    secondary.clone(),
                                    &vec![account],
                                    vec![block],
                                )
                                .await
                                .unwrap();
                            }
                        }
                    }
                    None => {}
                }
            }
            sleep(RESCHEDULING_DURATION).await;
        }
    });
    let _ = handler.await;

    let _ = tokio::join!(
        dworker_handler,
        status_update_handler,
        scheduling_handler,
        secondary_scheduling_handler
    );
    std::future::pending::<()>().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::blocks_range;

    #[test]
    fn block_range_test() {
        let range = blocks_range(1..100, 30);
        println!("ranges, {:?}", range);
    }
}
