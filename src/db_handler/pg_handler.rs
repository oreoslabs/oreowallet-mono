use futures::executor::block_on;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::error::OreoError;

use super::{Account, DBHandler, UnstableAccount};

#[derive(Debug, Clone)]
pub struct PgHandler {
    pub pool: PgPool,
}

impl PgHandler {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, account: Account) -> Result<String, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO wallet.account (name, create_head, create_hash, head, hash, in_vk, out_vk, vk, address) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING name"
        )
        .bind(account.name.clone())
        .bind(account.create_head.clone())
        .bind(account.create_hash.clone())
        .bind(account.head)
        .bind(account.hash.clone())
        .bind(account.in_vk.clone())
        .bind(account.out_vk.clone())
        .bind(account.vk.clone())
        .bind(account.address.clone())
        .fetch_one(&self.pool)
        .await?.get(0);
        Ok(result)
    }

    pub async fn insert_primary(&self, account: UnstableAccount) -> Result<String, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO wallet.primarychain (sequence, hash, address) VALUES ($1, $2, $3) RETURNING address"
        )
        .bind(account.sequence)
        .bind(account.hash.clone())
        .bind(account.address.clone())
        .fetch_one(&self.pool)
        .await?.get(0);
        Ok(result)
    }

    pub async fn get_one(&self, address: String) -> Result<Account, sqlx::Error> {
        let result =
            sqlx::query_as::<_, Account>("SELECT * FROM wallet.account WHERE address = $1")
                .bind(address)
                .fetch_one(&self.pool)
                .await?;
        Ok(result)
    }

    pub async fn get_one_by_name(&self, name: String) -> Result<Account, sqlx::Error> {
        let result = sqlx::query_as::<_, Account>("SELECT * FROM wallet.account WHERE name = $1")
            .bind(name)
            .fetch_one(&self.pool)
            .await?;
        Ok(result)
    }

    pub async fn update_one(&self, state: Account) -> Result<String, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE wallet.account SET head = $1, hash = $2 WHERE address = $3 RETURNING name",
        )
        .bind(state.head)
        .bind(state.hash.clone())
        .bind(state.address.clone())
        .fetch_one(&self.pool)
        .await?
        .get(0);
        Ok(result)
    }

    pub async fn delete(&self, address: String) -> Result<String, sqlx::Error> {
        let result = sqlx::query("DELETE FROM wallet.account WHERE address = $1 RETURNING name")
            .bind(address)
            .fetch_one(&self.pool)
            .await?
            .get(0);
        Ok(result)
    }

    pub async fn delete_primary(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<String, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM wallet.primarychain WHERE address = $1 AND sequence = $2 RETURNING address",
        )
        .bind(address)
        .bind(sequence)
        .fetch_one(&self.pool)
        .await?
        .get(0);
        Ok(result)
    }

    pub async fn find_many_with_oldest_head(&self) -> Result<Vec<Account>, sqlx::Error> {
        let result = sqlx::query_as(
            "SELECT * FROM wallet.account WHERE head = (SELECT MIN(head) FROM wallet.account)",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(result)
    }

    pub async fn find_many_with_head_filter(&self, head: i64) -> Result<Vec<Account>, sqlx::Error> {
        let result = sqlx::query_as("SELECT * FROM wallet.account WHERE head >= $1")
            .bind(head)
            .fetch_all(&self.pool)
            .await?;
        Ok(result)
    }

    pub async fn get_one_from_primary(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<UnstableAccount, sqlx::Error> {
        let result = sqlx::query_as::<_, UnstableAccount>(
            "SELECT * FROM wallet.primarychain WHERE address = $1 AND sequence = $2",
        )
        .bind(address)
        .bind(sequence)
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }
}

#[async_trait::async_trait]
impl DBHandler for PgHandler {
    fn from_config(config: &crate::config::DbConfig) -> Self {
        let url = config.server_url();
        let pool = block_on(async {
            PgPoolOptions::default()
                .max_connections(config.default_pool_size)
                .connect(&url)
                .await
                .unwrap()
        });
        Self::new(pool)
    }

    async fn save_account(&self, account: Account, _worker_id: u32) -> Result<String, OreoError> {
        let old_account = self.get_one(account.address.clone()).await;
        match old_account {
            Ok(_) => Err(OreoError::Duplicate(account.address)),
            Err(e) => match e {
                sqlx::Error::RowNotFound => {
                    self.insert(account).await.map_err(|_| OreoError::DBError)
                }
                _ => Err(OreoError::DBError),
            },
        }
    }

    async fn get_account(&self, address: String) -> Result<Account, OreoError> {
        self.get_one(address.clone()).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => OreoError::NoImported(address),
            _ => OreoError::DBError,
        })
    }

    async fn remove_account(&self, address: String) -> Result<String, OreoError> {
        self.delete(address.clone()).await.map_err(|e| match e {
            sqlx::Error::RowNotFound => OreoError::NoImported(address),
            _ => OreoError::DBError,
        })
    }

    async fn update_account_head(
        &self,
        address: String,
        new_head: i64,
        new_hash: String,
    ) -> Result<String, OreoError> {
        match self.get_one(address.clone()).await {
            Ok(mut account) => {
                account.head = new_head;
                account.hash = new_hash;
                self.update_one(account).await.map_err(|e| match e {
                    sqlx::Error::RowNotFound => OreoError::NoImported(address),
                    _ => OreoError::DBError,
                })
            }
            Err(_) => Err(OreoError::NoImported(address)),
        }
    }

    async fn get_oldest_accounts(&self) -> Result<Vec<Account>, OreoError> {
        self.find_many_with_oldest_head()
            .await
            .map_err(|_| OreoError::DBError)
    }

    async fn get_accounts_with_head(&self, start_head: i64) -> Result<Vec<Account>, OreoError> {
        self.find_many_with_head_filter(start_head)
            .await
            .map_err(|_| OreoError::DBError)
    }

    async fn get_primary_account(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<UnstableAccount, OreoError> {
        self.get_one_from_primary(address.clone(), sequence)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => OreoError::NoImported(address),
                _ => OreoError::DBError,
            })
    }

    async fn del_primary_account(
        &self,
        address: String,
        sequence: i64,
    ) -> Result<String, OreoError> {
        self.delete_primary(address.clone(), sequence)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => OreoError::NoImported(address),
                _ => OreoError::DBError,
            })
    }

    async fn add_primary_account(&self, account: UnstableAccount) -> Result<String, OreoError> {
        self.insert_primary(account)
            .await
            .map_err(|_| OreoError::DBError)
    }
}

#[cfg(test)]
mod tests {
    use sqlx_db_tester::TestDb;

    use crate::{
        constants::{MAINNET_GENESIS_HASH, MAINNET_GENESIS_SEQUENCE},
        db_handler::{address_to_name, Account, DBHandler},
    };

    use super::PgHandler;

    const VK: &str = "4ae4eb9606ba57b3b17a444100a9ac6453cd67e6fe4c860e63a2e18b1200978ab5ecce68e8639d5016cbe73b0ea9a3c8e906fc881af2e9ccfa7a7b63fb73d555";
    const IN_VK: &str = "4a08bec0ec5a471352f340d737e4b3baec2aec8d0a2e12201d92d8ad71aadd07";
    const OUT_VK: &str = "cee4ff41d7d8da5eedc6493134981eaad7b26a8b0291a4eac9ba95090fa47bf7";
    const ADDRESS: &str = "d63ba13d7c35caf942c64d5139b948b885ec931977a3f248c13e7f3c1bd0aa64";

    fn get_tdb() -> TestDb {
        TestDb::new("localhost", 5432, "postgres", "postgres", "./migrations")
    }

    fn get_test_account() -> Account {
        Account {
            name: address_to_name(ADDRESS),
            create_head: None,
            create_hash: None,
            head: MAINNET_GENESIS_SEQUENCE,
            hash: MAINNET_GENESIS_HASH.to_string(),
            in_vk: IN_VK.to_string(),
            out_vk: OUT_VK.to_string(),
            vk: VK.to_string(),
            address: ADDRESS.to_string(),
        }
    }

    #[tokio::test]
    async fn save_account_should_work_pg() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let pg_handler = PgHandler::new(pool);
        let account = get_test_account();
        let saved = pg_handler.save_account(account, 0).await;
        assert!(saved.is_ok());
        let saved = saved.unwrap();
        assert_eq!(ADDRESS.to_string(), saved);
    }

    #[tokio::test]
    async fn get_account_should_work_pg() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let pg_handler = PgHandler::new(pool);
        let account = get_test_account();
        let saved = pg_handler.save_account(account.clone(), 0).await.unwrap();
        let saved = pg_handler.get_account(saved).await;
        assert!(saved.is_ok());
        let saved = saved.unwrap();
        assert_eq!(account, saved);
    }

    #[tokio::test]
    async fn remove_account_should_work_pg() {
        let tdb = get_tdb();
        let pool = tdb.get_pool().await;
        let pg_handler = PgHandler::new(pool);
        let account = get_test_account();
        let saved = pg_handler.save_account(account.clone(), 0).await.unwrap();
        let result = pg_handler.remove_account(saved.clone()).await;
        assert!(result.is_ok());

        // run remove once again
        let should_error = pg_handler.remove_account(saved).await;
        assert!(should_error.is_err());
    }
}
