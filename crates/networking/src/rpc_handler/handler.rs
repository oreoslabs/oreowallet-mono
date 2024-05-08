use std::{fmt::Debug, time::Duration};

use oreo_errors::OreoError;
use serde::Deserialize;
use serde_json::json;
use tracing::debug;
use ureq::{Agent, AgentBuilder, Error, Response};

use crate::{
    rpc_abi::{
        RpcAddTransactionRequest, RpcBroadcastTxRequest, RpcBroadcastTxResponse,
        RpcCreateTxRequest, RpcCreateTxResponse, RpcExportAccountResponse,
        RpcGetAccountStatusRequest, RpcGetAccountStatusResponse, RpcGetAccountTransactionRequest,
        RpcGetAccountTransactionResponse, RpcGetBalancesRequest, RpcGetBalancesResponse,
        RpcGetBlockRequest, RpcGetBlockResponse, RpcGetBlocksRequest, RpcGetBlocksResponse,
        RpcGetLatestBlockResponse, RpcGetTransactionsRequest, RpcGetTransactionsResponse,
        RpcImportAccountRequest, RpcImportAccountResponse, RpcRemoveAccountRequest,
        RpcRemoveAccountResponse, RpcResponse, RpcStartSyncingResponse, RpcStopSyncingResponse,
    },
    rpc_handler::RpcError,
};

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

    pub async fn import_account(
        &self,
        request: RpcImportAccountRequest,
    ) -> Result<RpcResponse<RpcImportAccountResponse>, OreoError> {
        let path = format!("http://{}/wallet/importAccount", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(ureq::json!({"account": request}));
        handle_response(resp)
    }

    pub async fn export_account(
        &self,
        account: String,
    ) -> Result<RpcResponse<RpcExportAccountResponse>, OreoError> {
        let path = format!("http://{}/wallet/exportAccount", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(json!({"account": account, "format": "JSON".to_string()}));
        handle_response(resp)
    }

    pub async fn remove_account(
        &self,
        request: RpcRemoveAccountRequest,
    ) -> Result<RpcResponse<RpcRemoveAccountResponse>, OreoError> {
        debug!("req: {:?}", request);
        let path = format!("http://{}/wallet/removeAccount", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(&request)
            .map(|res| match res.status() {
                200 => Response::new(200, "OK", "{\"status\":200,\"data\":{\"removed\":true}}")
                    .unwrap(),
                _ => res,
            });
        handle_response(resp)
    }

    pub async fn get_account_status(
        &self,
        request: RpcGetAccountStatusRequest,
    ) -> Result<RpcResponse<RpcGetAccountStatusResponse>, OreoError> {
        let path = format!("http://{}/wallet/getAccountStatus", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn stop_syncing(
        &self,
        request: RpcGetAccountStatusRequest,
    ) -> Result<RpcResponse<RpcStopSyncingResponse>, OreoError> {
        let path = format!("http://{}/wallet/stopSyncing", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn start_syncing(
        &self,
        request: RpcGetAccountStatusRequest,
    ) -> Result<RpcResponse<RpcStartSyncingResponse>, OreoError> {
        let path = format!("http://{}/wallet/startSyncing", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn add_transaction(
        &self,
        request: RpcAddTransactionRequest,
    ) -> Result<RpcResponse<RpcBroadcastTxResponse>, OreoError> {
        let path = format!("http://{}/wallet/addTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn get_balances(
        &self,
        request: RpcGetBalancesRequest,
    ) -> Result<RpcResponse<RpcGetBalancesResponse>, OreoError> {
        let path = format!("http://{}/wallet/getBalances", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn get_account_transaction(
        &self,
        request: RpcGetAccountTransactionRequest,
    ) -> Result<RpcResponse<RpcGetAccountTransactionResponse>, OreoError> {
        let path = format!("http://{}/wallet/getAccountTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn get_transactions(
        &self,
        request: RpcGetTransactionsRequest,
    ) -> Result<RpcResponse<RpcGetTransactionsResponse>, OreoError> {
        let path = format!("http://{}/wallet/getAccountTransactions", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn create_transaction(
        &self,
        request: RpcCreateTxRequest,
    ) -> Result<RpcResponse<RpcCreateTxResponse>, OreoError> {
        let path = format!("http://{}/wallet/createTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn broadcast_transaction(
        &self,
        request: RpcBroadcastTxRequest,
    ) -> Result<RpcResponse<RpcBroadcastTxResponse>, OreoError> {
        let path = format!("http://{}/chain/broadcastTransaction", self.endpoint);
        let resp = self.agent.clone().post(&path).send_json(&request);
        handle_response(resp)
    }

    pub async fn get_latest_block(
        &self,
    ) -> Result<RpcResponse<RpcGetLatestBlockResponse>, OreoError> {
        let path = format!("http://{}/chain/getChainInfo", self.endpoint);
        let resp = self.agent.clone().get(&path).call();
        handle_response(resp)
    }

    pub async fn get_block(
        &self,
        sequence: i64,
    ) -> Result<RpcResponse<RpcGetBlockResponse>, OreoError> {
        let path = format!("http://{}/chain/getBlock", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(RpcGetBlockRequest {
                sequence,
                serialized: Some(true),
            });
        handle_response(resp)
    }

    pub async fn get_blocks(
        &self,
        start: u64,
        end: u64,
    ) -> Result<RpcResponse<RpcGetBlocksResponse>, OreoError> {
        let path = format!("http://{}/chain/getBlocks", self.endpoint);
        let resp = self
            .agent
            .clone()
            .post(&path)
            .send_json(RpcGetBlocksRequest { start, end });
        handle_response(resp)
    }
}

pub fn handle_response<S: Debug + for<'a> Deserialize<'a>>(
    resp: Result<Response, Error>,
) -> Result<RpcResponse<S>, OreoError> {
    let res = match resp {
        Ok(response) => match response.into_json::<RpcResponse<S>>() {
            Ok(data) => Ok(data),
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
            message: e.to_string(),
        }),
    };
    debug!("Handle rpc response: {:?}", res);
    match res {
        Ok(data) => Ok(data),
        Err(e) => Err(OreoError::try_from(e).unwrap()),
    }
}
