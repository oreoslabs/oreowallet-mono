use std::net::SocketAddr;

use anyhow::Result;
use clap::Parser;
use db_handler::load_db;
use dotenv::dotenv;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let sk = std::env::var("SECRET_KEY").expect("SECRET_KEY not provided in env");
    let pk = std::env::var("PUBLIC_KEY").expect("PUBLIC_KEY not provided in env");
    let mut sk_u8 = [0u8; 32];
    let mut pk_u8 = [0u8; 33];
    let _ = hex::decode_to_slice(sk, &mut sk_u8).unwrap();
    let _ = hex::decode_to_slice(pk, &mut pk_u8).unwrap();
    let args = Command::parse();
    let Command {
        dlisten,
        restful,
        verbosity,
        dbconfig,
        node,
        server,
        network,
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
                sk_u8,
                pk_u8,
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
                sk_u8,
                pk_u8,
            )
            .await?;
        }
        _ => panic!("Invalid network used"),
    }
    Ok(())
}
