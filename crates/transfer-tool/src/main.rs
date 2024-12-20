use anyhow::Result;
use clap::Parser;
use db_handler::{DBHandler, DbConfig, PgHandler};
use networking::{
    rpc_abi::{OutPut, SendTransactionRequest},
    rpc_handler::RpcHandler,
};
use params::{mainnet::Mainnet, network::Network, testnet::Testnet};
use tracing::info;
use utils::{handle_signals, initialize_logger, EnvFilter};
#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The path to db config file
    #[clap(long)]
    pub dbconfig: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
    /// The account used to transfer from
    #[clap(long)]
    pub account: String,
    /// The bonus amount
    #[clap(long)]
    pub bonus: String,
    /// The network id, 0 for mainnet, 1 for testnet.
    #[clap(long)]
    pub network: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Command::parse();
    let Command {
        dbconfig,
        verbosity,
        node,
        account,
        bonus,
        network,
    } = cli;
    initialize_logger(verbosity, EnvFilter::from_default_env());
    handle_signals().await?;
    let db_config = DbConfig::load(dbconfig).unwrap();
    let db_handler = PgHandler::from_config(&db_config);
    let rpc_handler = RpcHandler::new(node);
    let asset_id = match network {
        Testnet::ID => Testnet::NATIVE_ASSET_ID.to_string(),
        _ => Mainnet::NATIVE_ASSET_ID.to_string(),
    };
    match db_handler.get_unpaid_addresses().await {
        Ok(accounts) => {
            info!("bonus accounts: {:?}", accounts);
            let mut outputs = vec![];
            for account in accounts.iter() {
                if account.paid {
                    continue;
                }
                outputs.push(OutPut {
                    public_address: account.address.clone(),
                    amount: bonus.clone(),
                    memo: Some("OreoWallet-Bonus".to_string()),
                    memo_hex: Some("".into()),
                    asset_id: Some(asset_id.clone()),
                });
            }
            if outputs.is_empty() {
                info!("no new bonus address");
                return Ok(());
            }
            let send_request = SendTransactionRequest {
                account: account.clone(),
                fee: "10".to_string(),
                expiration_delta: 30,
                outputs,
            };
            let result = rpc_handler.send_transaction(send_request);
            match result {
                Ok(result) => {
                    info!("transaction sent, {}", result.data.hash);
                    for account in accounts.iter() {
                        let _ = db_handler
                            .update_firstseen_status(account.address.clone())
                            .await;
                    }
                }
                Err(e) => {
                    tracing::error!("failed to send transaction, {:?}", e);
                }
            }
        }
        Err(e) => tracing::error!("failed to get accounts, {:?}", e),
    }
    Ok(())
}
