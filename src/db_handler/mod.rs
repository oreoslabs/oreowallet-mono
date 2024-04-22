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
    async fn save_account(&self, account: Account, worker_id: u32) -> Result<String, OreoError>;
    /// Get account name from db
    async fn get_account(&self, address: String) -> Result<Account, OreoError>;
    /// Remove account from db
    async fn remove_account(&self, address: String) -> Result<String, OreoError>;
    /// Update account head in db
    async fn update_account_head(
        &self,
        address: String,
        new_head: i64,
        new_hash: String,
    ) -> Result<String, OreoError>;
    /// Get accounts with oldest head
    async fn get_oldest_accounts(&self) -> Result<Vec<Account>, OreoError>;
    /// Get accounts with head filter
    async fn get_accounts_with_head(&self, start_head: i64) -> Result<Vec<Account>, OreoError>;
    /// Get account from primary table (unstable)
    async fn get_primary_account(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<UnstableAccount, OreoError>;
    /// Delete account info in primary table (unstable)
    async fn del_primary_account(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<String, OreoError>;
    /// Add account info to primary table (unstable)
    async fn add_primary_account(&self, account: UnstableAccount) -> Result<String, OreoError>;
    /// Update account created info
    async fn update_account__createdhead(
        &self,
        address: String,
        new_head: i64,
        new_hash: String,
    ) -> Result<String, OreoError>;
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct UnstableAccount {
    pub address: String,
    pub sequence: i64,
    pub hash: String,
}
