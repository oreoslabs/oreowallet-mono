pub mod codec;

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use tokio::{
    io::split,
    net::TcpStream,
    sync::{
        mpsc::{self, Sender},
        oneshot, RwLock,
    },
    time::timeout,
};
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info, warn};

use crate::{
    db_handler::{address_to_name, DBHandler, PgHandler},
    manager::codec::DMessage,
    rpc_handler::abi::ImportTransactionReq,
    SharedState,
};

use self::codec::{DMessageCodec, DRequest, DResponse};

#[derive(Debug, Clone)]
pub struct ServerWorker {
    pub router: Sender<DRequest>,
    // 1: Idle; 2: Busy
    pub status: u8,
}

impl ServerWorker {
    pub fn new(router: Sender<DRequest>) -> Self {
        Self { router, status: 1 }
    }
}

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub timestampt: i64,
    pub hash: String,
    pub sequence: i64,
}

#[derive(Debug, Clone)]
pub struct Manager {
    pub workers: Arc<RwLock<HashMap<String, ServerWorker>>>,
    pub task_queue: Arc<RwLock<Vec<DRequest>>>,
    pub task_mapping: Arc<RwLock<HashMap<String, TaskInfo>>>,
    pub shared: Arc<SharedState<PgHandler>>,
}

impl Manager {
    pub fn new(shared: Arc<SharedState<PgHandler>>) -> Arc<Self> {
        Arc::new(Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            task_queue: Arc::new(RwLock::new(vec![])),
            task_mapping: Arc::new(RwLock::new(HashMap::new())),
            shared,
        })
    }

    pub async fn handle_stream(stream: TcpStream, server: Arc<Self>) -> Result<()> {
        let (tx, mut rx) = mpsc::channel::<DRequest>(1024);
        let mut worker_name = stream.peer_addr().unwrap().clone().to_string();
        let (r, w) = split(stream);
        let mut outbound_w = FramedWrite::new(w, DMessageCodec::default());
        let mut outbound_r = FramedRead::new(r, DMessageCodec::default());
        let (router, handler) = oneshot::channel();
        let mut timer = tokio::time::interval(Duration::from_secs(300));
        let _ = timer.tick().await;

        let worker_server = server.clone();
        tokio::spawn(async move {
            let _ = router.send(());
            loop {
                tokio::select! {
                    _ = timer.tick() => {
                        debug!("no message from worker {} for 5 mins, exit", worker_name);
                        let _ = worker_server.workers.write().await.remove(&worker_name).unwrap();
                        break;
                    },
                    Some(request) = rx.recv() => {
                        debug!("new message from rx for worker {}", worker_name);
                        let _ = worker_server.workers.write().await.get_mut(&worker_name).unwrap().status = 2;
                        let send_future = outbound_w.send(DMessage::DRequest(request));
                        if let Err(error) = timeout(Duration::from_millis(200), send_future).await {
                            debug!("send message to worker timeout: {}", error);
                        }

                    },
                    result = outbound_r.next() => {
                        debug!("new message from outboud_reader {:?} of worker {}", result, worker_name);
                        match result {
                            Some(Ok(message)) => {
                                timer.reset();
                                match message {
                                    DMessage::RegisterWorker(register) => {
                                        debug!("heart beat info {:?}", register);
                                        match worker_name == register.name {
                                            true => {},
                                            false => {
                                                let mut worker = ServerWorker::new(tx.clone());
                                                worker_name = register.name;
                                                info!("new worker: {}", worker_name.clone());
                                                match worker_server.task_queue.write().await.pop() {
                                                    Some(task) => {
                                                        let _ = tx.send(task).await.unwrap();
                                                        worker.status = 2;
                                                    },
                                                    None => {},
                                                }
                                                let _ = worker_server.workers.write().await.insert(worker_name.clone(), worker);
                                            }
                                        }
                                    },
                                    DMessage::DRequest(_) => error!("invalid message from worker, should never happen"),
                                    DMessage::DResponse(response) => {
                                        let _ = worker_server.update_account(response).await;
                                        match worker_server.task_queue.write().await.pop() {
                                            Some(task) => {
                                                let _ = tx.send(task).await.unwrap();
                                            },
                                            None => worker_server.workers.write().await.get_mut(&worker_name).unwrap().status = 1,
                                        }
                                    },
                                }
                            },
                            _ => {
                                warn!("unknown message");
                                let _ = worker_server.workers.write().await.remove(&worker_name).unwrap();
                                break;
                            },
                        }
                    }
                }
            }
            error!("worker {} main loop exit", worker_name);
        });
        let _ = handler.await;
        Ok(())
    }

    pub async fn update_account(&self, response: DResponse) -> Result<()> {
        let DResponse { id, data, address } = response;
        let account = address_to_name(&address);
        let mapping = self.task_mapping.read().await;
        let block_info = mapping.get(&id);
        if block_info.is_none() {
            // this can be caused by unexpected restart things.
            error!("task_id missed in task_mapping");
            return Ok(());
        }
        let block_info = block_info.unwrap();
        let block_hash = block_info.hash.to_string();
        let sequence = block_info.sequence;
        // we should have only one account in single task
        // all account in DResponse.data should be the same one
        if data.is_empty() {
            let res = self
                .shared
                .rpc_handler
                .update_head(account.clone(), block_hash.clone())
                .await;
            match res {
                Ok(res) => {
                    if res.data.updated {
                        let _ = self
                            .shared
                            .db_handler
                            .update_account_head(address.clone(), sequence, block_hash.clone())
                            .await;
                        let _ = self.task_mapping.write().await.remove(&id).unwrap();
                        debug!("account head updated, {}", account);
                    } else {
                        error!("failed to update account head in node");
                    }
                }
                Err(e) => error!("failed to update account head, {:?}", e),
            }
            return Ok(());
        }

        for tx_hash in data.iter() {
            let imported = self
                .shared
                .rpc_handler
                .import_transaction(ImportTransactionReq {
                    account: account.clone(),
                    block_hash: block_hash.clone(),
                    transaction_hash: tx_hash.to_string(),
                })
                .await;
            match imported {
                Ok(raw) => {
                    if raw.data.imported {
                        let _ = self
                            .shared
                            .db_handler
                            .update_account_head(address.clone(), sequence, block_hash.clone())
                            .await;
                        let _ = self.task_mapping.write().await.remove(&id).unwrap();
                        debug!(
                            "transaction {} of account {} imported successfully",
                            tx_hash, account
                        );
                    } else {
                        error!("failed to import transaction in node");
                    }
                }
                Err(e) => {
                    error!("failed to import transaction, {:?}", e);
                }
            }
        }
        Ok(())
    }
}
