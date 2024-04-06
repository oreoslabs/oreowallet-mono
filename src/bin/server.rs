use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use ironfish_server::{
    config::DbConfig,
    db_handler::{DBHandler, PgHandler},
    handle_signals, initialize_logger, run_server,
};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The ip:port server will listen on for restful api
    #[clap(short, long, default_value = "0.0.0.0:10001")]
    pub listen: SocketAddr,
    /// The ip:port server will listen on for dworker
    #[clap(long, default_value = "0.0.0.0:10002")]
    pub dlisten: SocketAddr,
    /// The path to db config file
    #[clap(short, long)]
    pub config: String,
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
        dlisten,
        config,
        verbosity,
        node,
    } = args;
    initialize_logger(verbosity);
    handle_signals().await?;
    let db_config = DbConfig::load(config).unwrap();
    let db_handler = PgHandler::from_config(&db_config);
    run_server(listen.into(), node, db_handler, dlisten.into()).await?;
    Ok(())
}
