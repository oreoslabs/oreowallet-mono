use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub status: u16,
    pub data: T,
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
pub struct CreateTxReq {
    pub account: String,
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
    pub limit: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    pub hash: String,
    pub r#type: String,
    pub status: String,
    pub block_sequence: Option<u64>,
}
