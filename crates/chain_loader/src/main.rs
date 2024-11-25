use anyhow::Result;
use chain_loader::load_checkpoint;
use clap::Parser;
use db_handler::{DBHandler, DbConfig, PgHandler};
use utils::{handle_signals, initialize_logger};

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let Command {
        dbconfig,
        verbosity,
        node,
    } = args;
    initialize_logger(verbosity);
    handle_signals().await?;
    let db_config = DbConfig::load(dbconfig).unwrap();
    let db_handler = PgHandler::from_config(&db_config);
    load_checkpoint(node, db_handler).await?;
    Ok(())
}
