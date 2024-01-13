use std::time::Duration;

use anyhow::Result;
use ureq::{Agent, AgentBuilder};

use crate::web_handlers::abi::{GetAccountStatusRep, GetAccountStatusReq, GetLatestBlockRep};

use super::abi::*;

#[derive(Debug, Clone)]
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
        let path = format!("http://{}/wallet/importAccount", self.endpoint);
        let resp: RpcResponse<ImportAccountRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!({"account": req}))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn get_balance(&self, req: GetBalancesReq) -> Result<GetBalancesRep> {
        let path = format!("http://{}/wallet/getBalances", self.endpoint);
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
        let path = format!("http://{}/wallet/getAccountTransactions", self.endpoint);
        let resp: RpcResponse<Vec<TransactionStatus>> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn create_transaction(&self, req: CreateTxReq) -> Result<CreateTxRep> {
        let path = format!("http://{}/wallet/createTransaction", self.endpoint);
        let resp: RpcResponse<CreateTxRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn broadcast_transaction(&self, req: BroadcastTxReq) -> Result<BroadcastTxRep> {
        let path = format!("http://{}/chain/broadcastTransaction", self.endpoint);
        let resp: RpcResponse<BroadcastTxRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn get_account_status(
        &self,
        req: GetAccountStatusReq,
    ) -> Result<GetAccountStatusRep> {
        let path = format!("http://{}/wallet/getAccountStatus", self.endpoint);
        let resp: RpcResponse<GetAccountStatusRep> = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!(req))?
            .into_json()?;
        Ok(resp.data)
    }

    pub async fn get_latest_block(&self) -> Result<GetLatestBlockRep> {
        let path = format!("http://{}/chain/getChainInfo", self.endpoint);
        let resp: RpcResponse<GetLatestBlockRep> =
            self.agent.clone().get(&path).call()?.into_json()?;
        Ok(resp.data)
    }
}
