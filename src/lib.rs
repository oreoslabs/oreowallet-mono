use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use axum::{routing::post, Router};
use db_handler::{DBHandler, RedisClient};
use rpc_handler::RpcHandler;
use tokio::{net::TcpListener, sync::oneshot};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;
use web_handlers::import_view_only_handler;

pub mod db_handler;
pub mod rpc_handler;
pub mod web_handlers;

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
            db_handler,
            rpc_handler: RpcHandler::new(endpoint.into()),
        }
    }
}

pub async fn run_server(listen: SocketAddr, rpc_server: String, redis: String) -> Result<()> {
    let db_handler = RedisClient::init(&redis);
    let shared_state = Arc::new(Mutex::new(SharedState::new(db_handler, &rpc_server)));
    let router = Router::new()
        .route("/account", post(import_view_only_handler))
        .with_state(shared_state);
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
