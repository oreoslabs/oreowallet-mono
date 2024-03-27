use redis::{
    aio::MultiplexedConnection, AsyncCommands, Client, ErrorKind, FromRedisValue, RedisResult,
};
use std::collections::HashMap;
use substring::Substring;
use tracing::info;

use super::{Account, DBHandler};
use crate::{config::DbConfig, error::OreoError};

pub const REDIS_ACCOUNT_KEY: &str = "IRONACCOUNT";

#[derive(Debug, Clone)]
pub struct RedisClient {
    pub client: Client,
}

impl RedisClient {
    pub fn connect(url: &str, _max_connections: u32) -> RedisResult<Self> {
        let client = Client::open(url)?;
        Ok(Self { client })
    }

    pub async fn get_con(&self) -> RedisResult<MultiplexedConnection> {
        self.client.get_multiplexed_async_connection().await
    }

    pub async fn set_str(&self, key: &str, value: &str, ttl_seconds: i64) -> RedisResult<()> {
        let mut con = self.get_con().await?;
        con.set(key, value).await?;
        if ttl_seconds > 0 {
            con.expire(key, ttl_seconds).await?;
        }
        Ok(())
    }

    pub async fn hset(&self, key: &str, field: &str, value: &str) -> RedisResult<()> {
        let mut con = self.get_con().await?;
        con.hset(key, field, value).await
    }

    pub async fn hget(&self, key: &str, field: &str) -> RedisResult<String> {
        let mut con = self.get_con().await?;
        con.hget(key, field).await
    }

    pub async fn hgetall(&self, key: &str) -> RedisResult<HashMap<String, String>> {
        let mut con = self.get_con().await?;
        let val = con.hgetall(key).await?;
        FromRedisValue::from_redis_value(&val)
    }

    pub async fn hdel(&self, key: &str, field: &str) -> RedisResult<()> {
        let mut con = self.get_con().await?;
        con.hdel(key, field).await
    }

    pub async fn get_str(&self, key: &str) -> RedisResult<String> {
        let mut con = self.get_con().await?;
        let value = con.get(key).await?;
        FromRedisValue::from_redis_value(&value)
    }
}

#[async_trait::async_trait]
impl DBHandler for RedisClient {
    async fn save_account(&self, address: String, _worker_id: u32) -> Result<String, OreoError> {
        match self.hget(REDIS_ACCOUNT_KEY, &address).await {
            Ok(_) => {
                return Err(OreoError::Duplicate(address));
            }
            Err(e) => {
                if e.is_connection_dropped() {
                    return Err(OreoError::DBError);
                };
                info!("Ready to save new account, {}", address);
            }
        }
        let account_name = address_to_name(&address);
        if let Err(_) = self.hset(REDIS_ACCOUNT_KEY, &address, &account_name).await {
            return Err(OreoError::DBError);
        }
        info!(
            "New account saved in redis, name: {}, address: {}",
            account_name, address
        );
        Ok(account_name)
    }

    async fn get_account(&self, address: String) -> Result<Account, OreoError> {
        match self.hget(REDIS_ACCOUNT_KEY, &address).await {
            Ok(name) => Ok(Account::redis_mock(name)),
            Err(e) => match e.kind() {
                ErrorKind::TypeError => Err(OreoError::NoImported(address)),
                _ => Err(OreoError::DBError),
            },
        }
    }

    async fn remove_account(&self, address: String) -> Result<String, OreoError> {
        match self.hget(REDIS_ACCOUNT_KEY, &address).await {
            Ok(name) => {
                // should never panic
                self.hdel(REDIS_ACCOUNT_KEY, &address).await.unwrap();
                Ok(name)
            }
            Err(e) => match e.kind() {
                ErrorKind::TypeError => Err(OreoError::NoImported(address)),
                _ => Err(OreoError::DBError),
            },
        }
    }

    async fn get_accounts(&self, _filter_head: u32) -> Result<Vec<Account>, OreoError> {
        match self.hgetall(REDIS_ACCOUNT_KEY).await {
            Ok(accounts) => Ok(accounts
                .values()
                .into_iter()
                .map(|account| Account::redis_mock(account.to_string()))
                .collect()),
            Err(_) => Err(OreoError::DBError),
        }
    }

    fn from_config(config: &DbConfig) -> Self {
        info!("Redis handler selected");
        RedisClient::connect(&config.server_url(), config.default_pool_size).unwrap()
    }
}

pub fn address_to_name(address: &str) -> String {
    address.substring(0, 10).into()
}

#[cfg(test)]
mod tests {

    // account used for tests
    //     Mnemonic  eight fog reward cat spoon lawsuit mention mean number wine female asthma adapt flush salad slam rib desert goddess flame code pass turn route
    //  Spending Key  46eb4ae291ed28fc62c44e977f7153870030b3af9658b8e77590ac22d1417ab5
    //      View Key  4ae4eb9606ba57b3b17a444100a9ac6453cd67e6fe4c860e63a2e18b1200978ab5ecce68e8639d5016cbe73b0ea9a3c8e906fc881af2e9ccfa7a7b63fb73d555
    //   Incoming View Key  4a08bec0ec5a471352f340d737e4b3baec2aec8d0a2e12201d92d8ad71aadd07
    //   Outgoing View Key  cee4ff41d7d8da5eedc6493134981eaad7b26a8b0291a4eac9ba95090fa47bf7
    //       Address  d63ba13d7c35caf942c64d5139b948b885ec931977a3f248c13e7f3c1bd0aa64

    use super::address_to_name;
    use super::RedisClient;
    use crate::config::DbConfig;
    use crate::db_handler::Account;
    use crate::db_handler::DBHandler;
    use crate::error::OreoError;

    const VK: &str = "4ae4eb9606ba57b3b17a444100a9ac6453cd67e6fe4c860e63a2e18b1200978ab5ecce68e8639d5016cbe73b0ea9a3c8e906fc881af2e9ccfa7a7b63fb73d555";
    const IN_VK: &str = "4a08bec0ec5a471352f340d737e4b3baec2aec8d0a2e12201d92d8ad71aadd07";
    const OUT_VK: &str = "cee4ff41d7d8da5eedc6493134981eaad7b26a8b0291a4eac9ba95090fa47bf7";
    const ADDRESS: &str = "d63ba13d7c35caf942c64d5139b948b885ec931977a3f248c13e7f3c1bd0aa64";

    #[tokio::test]
    async fn save_account_should_work() {
        let config = DbConfig::load("./fixtures/redis-config.yml").unwrap();
        let db_handler = RedisClient::from_config(&config);
        let account_name = address_to_name(ADDRESS);
        let saved_name = db_handler.save_account(ADDRESS.to_string(), 0).await;
        assert_eq!(account_name, saved_name.unwrap());
    }

    #[tokio::test]
    async fn get_account_should_work() {
        let config = DbConfig::load("./fixtures/redis-config.yml").unwrap();
        let db_handler = RedisClient::from_config(&config);
        let account_name = address_to_name(ADDRESS);
        let saved_name = db_handler.get_account(ADDRESS.to_string()).await.unwrap();
        assert_eq!(Account::redis_mock(account_name), saved_name);
    }

    #[tokio::test]
    async fn remove_account_should_work() {
        let config = DbConfig::load("./fixtures/redis-config.yml").unwrap();
        let db_handler = RedisClient::from_config(&config);
        let account_name = address_to_name(ADDRESS);
        let removed_name = db_handler
            .remove_account(ADDRESS.to_string())
            .await
            .unwrap();
        assert_eq!(account_name, removed_name);

        // this get_account should be error
        let should_error_account = db_handler.get_account(ADDRESS.to_string()).await;
        assert!(should_error_account.is_err());
        let should_error_account = should_error_account.err().unwrap();
        let expected = OreoError::NoImported(ADDRESS.to_string());
        assert_eq!(expected, should_error_account);
    }
}
