use anyhow::Result;
use clap::Parser;
use ironfish_server::{dworkers::start_worker, handle_signals, initialize_logger};
use std::net::SocketAddr;
use tracing::info;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// scheduler to connect to
    #[arg(short, long)]
    address: SocketAddr,
    /// worker name
    #[arg(short, long, default_value = "dworker")]
    name: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();
    initialize_logger(args.verbosity);
    handle_signals().await?;
    info!(
        "Start connecting to scheduler: {:?} with name {}",
        args.address, args.name
    );
    start_worker(args.address, args.name).await?;
    Ok(())
}
