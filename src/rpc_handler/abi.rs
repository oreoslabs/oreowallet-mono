use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub status: u16,
    pub data: T,
}

impl<T: Serialize> IntoResponse for RpcResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(json!({"code": 200, "data": self.data})).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateAccountOpt {
    pub hash: String,
    pub sequence: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAccountReq {
    pub version: u8,
    pub name: String,
    pub view_key: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub public_address: String,
    pub created_at: Option<CreateAccountOpt>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportAccountRep {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetBalancesReq {
    pub account: String,
    pub confirmations: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalance {
    pub asset_id: String,
    pub confirmed: String,
    pub unconfirmed: String,
    pub pending: String,
    pub available: String,
    pub sequence: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetBalancesRep {
    pub account: String,
    pub balances: Vec<AssetBalance>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutPut {
    pub public_address: String,
    pub amount: String,
    pub memo: String,
    pub asset_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTxReq {
    pub account: String,
    pub fee: Option<String>,
    pub expiration_delta: Option<u32>,
    pub outputs: Vec<OutPut>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateTxRep {
    pub transaction: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BroadcastTxReq {
    pub transaction: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BroadcastTxRep {
    pub hash: String,
    pub accepted: bool,
    pub broadcasted: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetTransactionsReq {
    pub account: String,
    pub limit: Option<u32>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    pub hash: String,
    pub fee: String,
    pub r#type: String,
    pub status: String,
    pub block_sequence: Option<u64>,
    pub timestamp: String,
    pub asset_balance_deltas: AssetBalanceDelta,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionsRep {
    pub transactions: Vec<TransactionStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetAccountTransactionReq {
    pub account: String,
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalanceDelta {
    pub asset_id: String,
    pub delta: String,
    pub asset_name: String,
}
#[derive(Debug, Deserialize, Serialize)]
pub struct GetAccountTransactionRep {
    pub account: String,
    pub transaction: Option<TransactionStatus>,
}
