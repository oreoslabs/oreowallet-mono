use anyhow::Result;
use clap::Parser;
use ironfish_server::{
    config::DbConfig,
    db_handler::{DBHandler, PgHandler, RedisClient, REDIS_ACCOUNT_KEY},
    handle_signals, initialize_logger,
    rpc_handler::{abi::ImportAccountReq, RpcHandler},
};

#[derive(Parser, Debug, Clone)]
pub struct Command {
    /// The path to source db config file (redis)
    #[clap(short, long)]
    pub config: String,
    /// Set your logger level
    #[clap(short, long, default_value = "0")]
    pub verbosity: u8,
    /// The Ironfish rpc node to connect to
    #[clap(short, long, default_value = "127.0.0.1:9092")]
    pub node: String,
    /// Destination db name, redis or postgres
    #[clap(long, default_value = "redis")]
    pub dname: String,
    /// Destination db config file
    #[clap(long)]
    pub dconfig: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let Command {
        config,
        verbosity,
        node,
        dname,
        dconfig,
    } = args;
    initialize_logger(verbosity);
    handle_signals().await?;
    let db_config = DbConfig::load(config).unwrap();
    let redis_handler = RedisClient::from_config(&db_config);
    let rpc_handler = RpcHandler::new(node.into());
    let accounts_v0 = redis_handler.hgetall(REDIS_ACCOUNT_KEY).await.unwrap();
    match dname.as_str() {
        "redis" => {
            for (_, name) in accounts_v0.into_iter() {
                if let Ok(imported) = rpc_handler.export_account(name).await {
                    let account = imported.data.account;
                    let account: ImportAccountReq = serde_json::from_str(&account).unwrap();
                    let _ = redis_handler
                        .save_account(account.to_account(), 0)
                        .await
                        .unwrap();
                }
            }
        }
        "postgres" => {
            let pg_handler = PgHandler::from_config(&DbConfig::load(dconfig).unwrap());
            for (_, name) in accounts_v0.into_iter() {
                if let Ok(imported) = rpc_handler.export_account(name).await {
                    let account = imported.data.account;
                    let account: ImportAccountReq = serde_json::from_str(&account).unwrap();
                    let _ = pg_handler
                        .save_account(account.to_account(), 0)
                        .await
                        .unwrap();
                }
            }
        }
        _ => unimplemented!(),
    }
    Ok(())
}
