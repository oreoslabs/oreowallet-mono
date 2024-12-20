use anyhow::Result;
use chain_loader::load_checkpoint;
use clap::Parser;
use db_handler::{DBHandler, DbConfig, PgHandler};
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use utils::{initialize_logger, EnvFilter};

#[derive(Parser, Debug, Clone)]
struct Command {
    /// The path to db config file
    #[clap(long)]
    pub dbconfig: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
    /// The network id, 0 for mainnet, 1 for testnet.
    #[clap(long)]
    pub network: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let Command {
        dbconfig,
        verbosity,
        node,
        network,
    } = args;
    let filter = EnvFilter::from_default_env();
    initialize_logger(verbosity, filter);
    let db_config = DbConfig::load(dbconfig).unwrap();
    let db_handler = PgHandler::from_config(&db_config);
    match network {
        Mainnet::ID => {
            load_checkpoint::<Mainnet>(node, db_handler).await?;
        }
        Testnet::ID => {
            load_checkpoint::<Testnet>(node, db_handler).await?;
        }
        _ => panic!("Invalid network used"),
    }
    Ok(())
}
