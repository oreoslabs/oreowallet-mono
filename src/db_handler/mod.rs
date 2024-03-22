mod redis_handler;

pub use redis_handler::*;

use serde::{Deserialize, Serialize};

use crate::{
    constants::{MAINNET_GENESIS, MAINNET_SEQUENCE},
    error::OreoError,
};

pub trait DBHandler {
    /// Initialize a DB handler, redis only now
    fn init(db_addr: &str) -> Self;
    /// Save account in db and return account name
    fn save_account(&self, address: String, worker_id: u32) -> Result<String, OreoError>;
    /// Get account name from db
    fn get_account(&self, address: String) -> Result<Account, OreoError>;
    /// Remove account from db
    fn remove_account(&self, address: String) -> Result<String, OreoError>;
    /// Get all accounts
    fn get_accounts(&self, filter_head: u32) -> Result<Vec<Account>, OreoError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
    pub name: String,
    pub create_head: Option<u32>,
    pub create_hash: Option<String>,
    pub head: u32,
    pub hash: String,
    pub in_vk: String,
    pub out_vk: String,
    pub vk: String,
}

impl Account {
    /// used to make redis handler work
    pub fn redis_mock(name: String) -> Self {
        Self {
            name,
            create_head: None,
            create_hash: None,
            head: MAINNET_SEQUENCE,
            hash: MAINNET_GENESIS.into(),
            in_vk: "".into(),
            out_vk: "".into(),
            vk: "".into(),
        }
    }
}
