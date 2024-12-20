use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use prover::run_prover;
use tracing::info;
use utils::{initialize_logger, EnvFilter};

#[derive(Parser, Debug, Clone)]
pub struct Prover {
    /// The ip:port prover will listen on
    #[clap(short, long, default_value = "0.0.0.0:10002")]
    listen: SocketAddr,
    /// Set prover logger level
    #[clap(short, long, default_value = "0")]
    verbosity: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let prover = Prover::parse();
    let filter = EnvFilter::from_default_env().add_directive("bellperson=off".parse().unwrap());
    initialize_logger(prover.verbosity, filter);
    info!("Prover starts {:?}", prover);
    run_prover(prover.listen).await?;
    Ok(())
}
