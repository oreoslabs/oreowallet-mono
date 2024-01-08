mod redis_handler;

pub use redis_handler::*;

use anyhow::Result;

pub trait DBHandler {
    /// Initialize a DB handler, redis only now
    fn init(db_addr: &str) -> Self;
    /// Save account in db and return account name
    fn save_account(&self, address: String, worker_id: u32) -> Result<String>;
    /// Get account name from db
    fn get_account(&self, address: String) -> Result<String>;
}
