use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    constants::IRON_NATIVE_ASSET,
    orescriptions::{get_ores, is_ores_local, Ores},
};

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
pub struct AssetStatus {
    status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalance {
    pub asset_id: String,
    pub asset_name: String,
    pub confirmed: String,
    pub unconfirmed: String,
    pub pending: String,
    pub available: String,
    pub sequence: Option<u64>,
    pub asset_verification: AssetStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetBalancesRep {
    pub account: String,
    pub balances: Vec<AssetBalance>,
}

impl GetBalancesRep {
    pub fn verified_asset(base: Self) -> Self {
        Self {
            balances: base
                .balances
                .into_iter()
                .filter(|x| {
                    x.asset_verification.status == "verified".to_string()
                        || x.asset_id == IRON_NATIVE_ASSET
                        || x.asset_name
                            == "6f7265736372697074696f6e7300000000000000000000000000000000000000"
                                .to_string()
                })
                .collect::<Vec<AssetBalance>>(),
            ..base
        }
    }

    pub async fn ores(base: Self) -> Vec<Ores> {
        let mut result = vec![];
        for asset in base.balances.iter() {
            if !is_ores_local(asset) {
                continue;
            }
            if let Ok(ores) = get_ores(&asset.asset_id).await {
                result.push(ores);
            }
        }
        result
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OutPut {
    pub public_address: String,
    pub amount: String,
    pub memo: Option<String>,
    pub asset_id: Option<String>,
}

impl OutPut {
    pub fn from(base: OutPut) -> Self {
        let memo = Some(base.memo.unwrap_or("".into()));
        let asset_id = Some(base.asset_id.unwrap_or(IRON_NATIVE_ASSET.into()));
        Self {
            memo,
            asset_id,
            ..base
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MintAsset {
    pub asset_id: Option<String>,
    pub name: Option<String>,
    pub metadata: Option<String>,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BurnAsset {
    pub asset_id: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTxReq {
    pub account: String,
    pub fee: Option<String>,
    pub expiration_delta: Option<u32>,
    pub outputs: Option<Vec<OutPut>>,
    pub mints: Option<Vec<MintAsset>>,
    pub burns: Option<Vec<BurnAsset>>,
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
    pub timestamp: u64,
    pub asset_balance_deltas: Vec<AssetBalanceDelta>,
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
    pub notes: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetBalanceDelta {
    pub asset_id: String,
    pub delta: String,
    pub asset_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcNote {
    pub value: String,
    pub memo: String,
    pub sender: String,
    pub owner: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionWithNotes {
    pub hash: String,
    pub fee: String,
    pub r#type: String,
    pub status: String,
    pub block_sequence: Option<u64>,
    pub timestamp: u64,
    pub asset_balance_deltas: Vec<AssetBalanceDelta>,
    pub notes: Option<Vec<RpcNote>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetAccountTransactionRep {
    pub account: String,
    pub transaction: Option<TransactionWithNotes>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAsset {
    pub id: String,
    pub name: String,
    pub metadata: String,
    pub verification: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetNoteWitnessReq {
    pub index: u32,
    pub confirmations: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthPath {
    pub side: String,
    pub hash_of_sibling: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNoteWitnessRep {
    pub tree_size: u32,
    pub root_hash: String,
    pub auth_path: Vec<AuthPath>,
}
