use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use db_handler::load_db;
use dservice::run_dserver;
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use utils::{handle_signals, initialize_logger, initialize_logger_filter, EnvFilter};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The ip:port server will listen on for worker to connect
    #[clap(long, default_value = "0.0.0.0:10001")]
    pub dlisten: SocketAddr,
    /// The ip:port server will listen on for worker to connect
    #[clap(long, default_value = "0.0.0.0:20001")]
    pub restful: SocketAddr,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The path to db config file
    #[clap(long)]
    pub dbconfig: String,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
    /// The server to connect to
    #[clap(short, long, default_value = "127.0.0.1:9093")]
    pub server: String,
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
        dlisten,
        restful,
        verbosity,
        dbconfig,
        node,
        server,
        network,
        operator,
    } = args;
    initialize_logger(verbosity);
    initialize_logger_filter(EnvFilter::from_default_env());
    handle_signals().await?;
    let db_handler = load_db(dbconfig).unwrap();
    match network {
        Mainnet::ID => {
            run_dserver::<Mainnet>(
                dlisten.into(),
                restful.into(),
                node,
                db_handler,
                server,
                operator,
            )
            .await?;
        }
        Testnet::ID => {
            run_dserver::<Testnet>(
                dlisten.into(),
                restful.into(),
                node,
                db_handler,
                server,
                operator,
            )
            .await?;
        }
        _ => panic!("Invalid network used"),
    }
    Ok(())
}
