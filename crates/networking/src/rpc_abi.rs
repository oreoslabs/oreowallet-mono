use axum::{response::IntoResponse, Json};
use params::network::Network;
use serde::{Deserialize, Serialize};
use ureq::json;

use crate::orescriptions::{get_ores, is_ores_local, Ores};

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponse<T> {
    pub status: u16,
    pub data: T,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcResponseStream<T> {
    pub data: T,
}

impl<T: Serialize> IntoResponse for RpcResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(json!({"code": 200, "data": self.data})).into_response()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockInfo {
    pub hash: String,
    pub sequence: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatedAt {
    pub hash: String,
    pub sequence: u64,
    pub network_id: u8,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcImportAccountRequest {
    pub version: u8,
    pub name: String,
    pub view_key: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub public_address: String,
    pub created_at: Option<CreatedAt>,
    pub spending_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcImportAccountResponse {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcExportAccountResponse {
    pub account: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcRemoveAccountRequest {
    pub account: String,
    pub confirm: Option<bool>,
    pub wait: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcRemoveAccountResponse {
    pub removed: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetAccountStatusRequest {
    pub account: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountStatus {
    pub name: String,
    pub head: Option<BlockInfo>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetAccountStatusResponse {
    pub account: AccountStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcSetScanningRequest {
    pub account: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionWithHash {
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlockWithHash {
    pub hash: String,
    pub sequence: i64,
    pub transactions: Vec<TransactionWithHash>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RpcSetAccountHeadRequest {
    pub account: String,
    pub start: String,
    pub end: String,
    pub blocks: Vec<BlockWithHash>,
    pub scan_complete: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RpcSetAccountHeadRequestV2 {
    pub account: String,
    pub start: String,
    pub end: String,
    pub blocks: Vec<BlockWithHash>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResetAccountRequest {
    pub account: String,
    pub reset_created_at: Option<bool>,
    pub reset_scanning_enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBalancesRequest {
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
    pub decimals: Option<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBalancesResponse {
    pub account: String,
    pub balances: Vec<AssetBalance>,
}

impl RpcGetBalancesResponse {
    pub fn verified_asset<N: Network>(base: Self) -> Self {
        Self {
            balances: base
                .balances
                .into_iter()
                .filter(|x| {
                    x.asset_verification.status == "verified".to_string()
                        || x.asset_id == N::NATIVE_ASSET_ID.to_string()
                        || x.asset_name
                            == "6f7265736372697074696f6e7300000000000000000000000000000000000000"
                                .to_string()
                })
                .collect::<Vec<AssetBalance>>(),
            ..base
        }
    }

    pub async fn ores<N: Network>(base: Self) -> Vec<Ores> {
        let mut result = vec![];
        for asset in base.balances.iter() {
            if !is_ores_local::<N>(asset) {
                continue;
            }
            if let Ok(ores) = get_ores::<N>(&asset.asset_id).await {
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
    pub memo_hex: Option<String>,
    pub asset_id: Option<String>,
}

impl OutPut {
    pub fn from<N: Network>(base: OutPut) -> Self {
        let memo = Some(base.memo.unwrap_or("".into()));
        let memo_hex = Some(base.memo_hex.unwrap_or("".into()));
        let asset_id = Some(base.asset_id.unwrap_or(N::NATIVE_ASSET_ID.into()));
        Self {
            memo,
            memo_hex,
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
pub struct RpcCreateTxRequest {
    pub account: String,
    pub fee: Option<String>,
    pub expiration_delta: Option<u32>,
    pub outputs: Option<Vec<OutPut>>,
    pub mints: Option<Vec<MintAsset>>,
    pub burns: Option<Vec<BurnAsset>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcCreateTxResponse {
    pub transaction: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcAddTxRequest {
    pub transaction: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcAddTxResponse {
    pub hash: String,
    pub accepted: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetTransactionsRequest {
    pub account: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub reverse: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
pub struct RpcGetTransactionsResponse {
    pub transactions: Vec<TransactionStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetAccountTransactionRequest {
    pub account: String,
    pub hash: String,
    pub notes: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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
pub struct RpcGetAccountTransactionResponse {
    pub account: String,
    pub transaction: Option<TransactionWithNotes>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBlockRequest {
    pub sequence: i64,
    pub serialized: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockIdentifier {
    pub index: String,
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcGetLatestBlockResponse {
    pub current_block_identifier: BlockIdentifier,
    pub genesis_block_identifier: BlockIdentifier,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcEncryptedNote {
    pub hash: String,
    pub serialized: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcTransaction {
    pub hash: String,
    pub notes: Vec<RpcEncryptedNote>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlock {
    pub hash: String,
    pub sequence: u32,
    pub previous_block_hash: String,
    pub transactions: Vec<RpcTransaction>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBlockResponse {
    pub block: RpcBlock,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBlocksRequest {
    pub start: u64,
    pub end: u64,
    pub serialized: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockItem {
    pub block: RpcBlock,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RpcGetBlocksResponse {
    pub blocks: Vec<BlockItem>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendTransactionRequest {
    pub account: String,
    pub fee: String,
    pub expiration_delta: u32,
    pub outputs: Vec<OutPut>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SendTransactionResponse {
    pub account: String,
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAssetVerification {
    pub status: String,
    pub symbol: Option<String>,
    pub decimals: Option<u8>,
    pub logo_uri: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcAsset {
    pub id: String,
    pub verification: RpcAssetVerification,
}
