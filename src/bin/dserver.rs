use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use ironfish_server::{handle_signals, initialize_logger, run_dserver};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The ip:port server will listen on for restful api
    #[clap(long, default_value = "0.0.0.0:10001")]
    pub dlisten: SocketAddr,
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
        dlisten,
        verbosity,
        node,
    } = args;
    initialize_logger(verbosity);
    handle_signals().await?;
    run_dserver(dlisten.into(), node).await?;
    Ok(())
}
