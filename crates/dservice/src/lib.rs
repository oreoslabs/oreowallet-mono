use std::{net::SocketAddr, sync::Arc, time::Duration};

use db_handler::PgHandler;
use manager::{Manager, SharedState};
use tokio::{net::TcpListener, sync::oneshot, time::sleep};
use tracing::{debug, error, info};

pub mod manager;
pub mod router;

pub async fn run_dserver(
    dlisten: SocketAddr,
    rpc_server: String,
    db_handler: PgHandler,
) -> anyhow::Result<()> {
    let shared_resource = Arc::new(SharedState::new(db_handler, &rpc_server));
    let manager = Manager::new(shared_resource.clone());
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
            sleep(Duration::from_secs(10)).await;
        }
    });
    let _ = handler.await;

    // todo: task scheduling and rescheduling handler

    let _ = tokio::join!(dworker_handler, status_update_handler,);
    std::future::pending::<()>().await;
    Ok(())
}
