use std::time::Duration;

use anyhow::Result;
use ureq::{Agent, AgentBuilder};

use super::abi::*;

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

    pub async fn import_view_only(&self, req: ImportAccountReq) -> Result<ImportAccountRep> {
        let path = format!("{}/wallet/importAccount", self.endpoint);
        let resp: RpcResponse<ImportAccountRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn get_balance(&self, req: GetBalancesReq) -> Result<GetBalancesRep> {
        let path = format!("{}/wallet/getBalances", self.endpoint);
        let resp: RpcResponse<GetBalancesRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn get_transactions(
        &self,
        req: GetTransactionsReq,
    ) -> Result<Vec<TransactionStatus>> {
        let path = format!("{}/wallet/getAccountTransactions", self.endpoint);
        let resp: RpcResponse<Vec<TransactionStatus>> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn create_transaction(&self, req: CreateTxReq) -> Result<CreateTxRep> {
        let path = format!("{}/wallet/createTransaction", self.endpoint);
        let resp: RpcResponse<CreateTxRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn broadcast_transaction(&self, req: BroadcastTxReq) -> Result<BroadcastTxRep> {
        let path = format!("{}/chain/broadcastTransaction", self.endpoint);
        let resp: RpcResponse<BroadcastTxRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }
}
