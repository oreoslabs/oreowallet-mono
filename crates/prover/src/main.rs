use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use prover::run_prover;
use utils::initialize_logger;

#[derive(Parser, Debug, Clone)]
pub struct Prover {
    /// The ip:port server will listen on
    #[clap(short, long, default_value = "0.0.0.0:10002")]
    pub listen: SocketAddr,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let prover = Prover::parse();
    initialize_logger(prover.verbosity);
    run_prover(prover.listen.into()).await?;
    Ok(())
}
