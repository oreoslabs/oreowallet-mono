use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use ironfish_server::{handle_signals, initialize_logger, run_server};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The ip:port server will listen on
    #[clap(short, long, default_value = "0.0.0.0:10001")]
    pub listen: SocketAddr,
    /// The redis server to connect to
    #[clap(short, long, default_value = "redis://localhost")]
    pub redis: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let Command {
        listen,
        redis,
        verbosity,
        node,
    } = args;
    initialize_logger(verbosity);
    handle_signals().await?;
    run_server(listen.into(), node, redis).await?;
    Ok(())
}
