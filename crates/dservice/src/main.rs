use anyhow::Result;
use db_handler::load_db;
use dservice::run_dserver;
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use utils::{
    handle_signals, initialize_logger, initialize_logger_filter, EnvFilter, Parser, Scanner,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Scanner::parse();
    let Scanner {
        dlisten,
        restful,
        dbconfig,
        node,
        server,
        network,
        operator,
        verbosity,
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
