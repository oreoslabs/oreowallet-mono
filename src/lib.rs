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
    broadcast_transaction_handler, create_transaction_handler, get_balances_handler,
    get_transactions_handler, import_vk_handler,
};

pub mod config;
pub mod db_handler;
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
            db_handler,
            rpc_handler: RpcHandler::new(endpoint.into()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Store<T: DBHandler> {
    pub inner: Arc<Mutex<SharedState<T>>>,
}

pub async fn run_server(listen: SocketAddr, rpc_server: String, redis: String) -> Result<()> {
    let db_handler = RedisClient::init(&redis);
    let shared_state = Arc::new(Mutex::new(SharedState::new(db_handler, &rpc_server)));
    let router = Router::new()
        .route("/import", post(import_vk_handler))
        .route("/getBalances", post(get_balances_handler))
        .route("/getTransactions", post(get_transactions_handler))
        .route("/createTx", post(create_transaction_handler))
        .route("/broadcastTx", post(broadcast_transaction_handler))
        .with_state(Store {
            inner: shared_state,
        })
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
