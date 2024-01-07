use std::time::Duration;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use ureq::{Agent, AgentBuilder};

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub status: u16,
    pub data: T,
}

pub struct RpcHandler {
    pub endpoint: String,
    pub agent: Agent,
}

impl RpcHandler {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            agent: AgentBuilder::new()
                .timeout_read(Duration::from_secs(5))
                .timeout_write(Duration::from_secs(5))
                .build(),
        }
    }

    pub async fn import_view_only() -> Result<()> {
        unimplemented!();
    }

    pub async fn get_balance() -> Result<String> {
        unimplemented!()
    }

    pub async fn get_transactions() -> Result<()> {
        unimplemented!()
    }

    pub async fn create_transaction() -> Result<String> {
        unimplemented!()
    }

    pub async fn broadcast_transaction() -> Result<()> {
        unimplemented!()
    }
}
