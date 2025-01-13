use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use db_handler::load_db;
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use server::run_server;
use utils::{handle_signals, initialize_logger, initialize_logger_filter, EnvFilter};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The ip:port server will listen on for restful api
    #[clap(long, default_value = "0.0.0.0:10001")]
    pub listen: SocketAddr,
    /// The path to db config file
    #[clap(long)]
    pub dbconfig: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
    /// The scan server to connect to
    #[clap(long, default_value = "127.0.0.1:9093")]
    pub scan: String,
    /// The network id, 0 for mainnet, 1 for testnet.
    #[clap(long)]
    pub network: u8,
    /// The operator secret key for signing messages.
    #[clap(long)]
    pub operator: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let Command {
        listen,
        dbconfig,
        verbosity,
        node,
        scan,
        network,
        operator,
    } = args;
    initialize_logger(verbosity);
    let filter = EnvFilter::from_default_env()
        .add_directive("ureq=off".parse().unwrap())
        .add_directive("rustls=off".parse().unwrap());
    initialize_logger_filter(filter);
    handle_signals().await?;
    let db_handler = load_db(dbconfig).unwrap();
    match network {
        Mainnet::ID => {
            run_server::<Mainnet>(listen.into(), node, db_handler, scan, operator).await?;
        }
        Testnet::ID => {
            run_server::<Testnet>(listen.into(), node, db_handler, scan, operator).await?;
        }
        _ => panic!("Invalid network used"),
    }
    Ok(())
}
