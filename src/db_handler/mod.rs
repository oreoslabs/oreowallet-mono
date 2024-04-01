mod pg_handler;
mod redis_handler;

pub use pg_handler::*;
pub use redis_handler::*;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{config::DbConfig, error::OreoError};

#[async_trait::async_trait]
pub trait DBHandler {
    /// Initialize a DB handler
    fn from_config(config: &DbConfig) -> Self;
    /// Save account in db and return account name
    async fn save_account(&self, address: Account, worker_id: u32) -> Result<String, OreoError>;
    /// Get account name from db
    async fn get_account(&self, address: String) -> Result<Account, OreoError>;
    /// Remove account from db
    async fn remove_account(&self, address: String) -> Result<String, OreoError>;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub name: String,
    pub create_head: Option<i64>,
    pub create_hash: Option<String>,
    pub head: i64,
    pub hash: String,
    pub in_vk: String,
    pub out_vk: String,
    pub vk: String,
    pub address: String,
}
