use std::time::Duration;

use serde::Deserialize;
use ureq::{Agent, AgentBuilder, Error, Response};

use crate::web_handlers::abi::{GetAccountStatusRep, GetAccountStatusReq, GetLatestBlockRep};

use super::{abi::*, RpcError};

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

    pub fn handle_response<S: for<'a> Deserialize<'a>>(
        &self,
        resp: Result<Response, Error>,
    ) -> Result<S, RpcError> {
        match resp {
            Ok(response) => match response.into_json::<RpcResponse<S>>() {
                Ok(data) => Ok(data.data),
                Err(e) => Err(RpcError {
                    code: "Unknown".into(),
                    status: 606,
                    message: e.to_string(),
                }),
            },
            Err(ureq::Error::Status(_code, response)) => match response.into_json::<RpcError>() {
                Ok(data) => Err(data),
                Err(e) => Err(RpcError {
                    code: "Unknown".into(),
                    status: 606,
                    message: e.to_string(),
                }),
            },
            Err(e) => Err(RpcError {
                code: "Unknown".into(),
                status: 606,
                message: "e".to_string(),
            }),
        }
    }

    pub async fn import_view_only(
        &self,
        req: ImportAccountReq,
    ) -> Result<ImportAccountRep, RpcError> {
        let path = format!("http://{}/wallet/importAccount", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!({"account": req}));
        self.handle_response(resp)
    }

    pub async fn get_balance(&self, req: GetBalancesReq) -> Result<GetBalancesRep, RpcError> {
        let path = format!("http://{}/wallet/getBalances", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&req);
        self.handle_response(resp)
    }

    pub async fn get_transactions(
        &self,
        req: GetTransactionsReq,
    ) -> Result<Vec<TransactionStatus>, RpcError> {
        let path = format!("http://{}/wallet/getAccountTransactions", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&req);
        self.handle_response(resp)
    }

    pub async fn create_transaction(&self, req: CreateTxReq) -> Result<CreateTxRep, RpcError> {
        let path = format!("http://{}/wallet/createTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&req);
        self.handle_response(resp)
    }

    pub async fn broadcast_transaction(
        &self,
        req: BroadcastTxReq,
    ) -> Result<BroadcastTxRep, RpcError> {
        let path = format!("http://{}/chain/broadcastTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&req);
        self.handle_response(resp)
    }

    pub async fn get_account_status(
        &self,
        req: GetAccountStatusReq,
    ) -> Result<GetAccountStatusRep, RpcError> {
        let path = format!("http://{}/wallet/getAccountStatus", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&req);
        self.handle_response(resp)
    }

    pub async fn get_latest_block(&self) -> Result<GetLatestBlockRep, RpcError> {
        let path = format!("http://{}/chain/getChainInfo", self.endpoint);
        let resp = self.agent.clone().get(&path).call();
        self.handle_response(resp)
    }
}
