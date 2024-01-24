use serde::{Deserialize, Serialize};

use crate::{
    error::OreoError,
    rpc_handler::abi::{
        AssetBalanceDelta, CreateAccountOpt, GetAccountTransactionRep, RpcNote, TransactionStatus,
    },
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAccountReq {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateProofReq {
    pub spend_circuits: Vec<Vec<u8>>,
    pub output_circuits: Vec<Vec<u8>>,
    pub mint_asset_circuits: Vec<Vec<u8>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateProofRep {
    pub spend_proofs: Vec<Vec<u8>>,
    pub output_proofs: Vec<Vec<u8>>,
    pub mint_asset_proofs: Vec<Vec<u8>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetAccountStatusReq {
    pub account: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountStatus {
    pub name: String,
    pub head: Option<CreateAccountOpt>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetAccountStatusRep {
    pub account: AccountStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockIdentifier {
    pub index: String,
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLatestBlockRep {
    pub current_block_identifier: BlockIdentifier,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionDetail {
    pub hash: String,
    pub fee: String,
    pub r#type: String,
    pub status: String,
    pub block_sequence: Option<u64>,
    pub timestamp: u64,
    pub asset_balance_deltas: Vec<AssetBalanceDelta>,
    pub sender: String,
    pub receiver: String,
    pub memo: Option<String>,
    pub value: String,
}

impl TransactionDetail {
    pub fn from(notes: Vec<RpcNote>, tx: TransactionStatus) -> Result<Self, OreoError> {
        // we handle send/receive transactions only now
        // todo: miner transaction
        let status = match tx.r#type.as_str() {
            "send" => {
                let receiver_candi: Vec<RpcNote> = notes
                    .into_iter()
                    .filter(|note| note.owner != note.sender)
                    .collect();
                // currently, there should be only one receiver in most cases
                let receiver_note = &receiver_candi[0];
                let receiver = receiver_note.owner.to_owned();
                let sender = receiver_note.sender.to_owned();
                let memo = receiver_note.memo.to_owned();
                let value = receiver_note.value.to_owned();
                Some((sender, receiver, memo, value))
            }
            "receive" => {
                let sender_candi: Vec<RpcNote> = notes
                    .into_iter()
                    .filter(|note| note.owner != note.sender)
                    .collect();
                let sender_note = &sender_candi[0];
                let sender = sender_note.sender.to_owned();
                let memo = sender_note.memo.to_owned();
                let receiver = sender_note.owner.to_owned();
                let value = sender_note.value.to_owned();
                Some((sender, receiver, memo, value))
            }
            _ => None,
        };
        match status {
            Some((sender, receiver, memo, value)) => {
                let TransactionStatus {
                    hash,
                    fee,
                    r#type,
                    status,
                    block_sequence,
                    timestamp,
                    asset_balance_deltas,
                } = tx;
                Ok(Self {
                    hash,
                    fee,
                    r#type,
                    status,
                    block_sequence,
                    timestamp,
                    asset_balance_deltas,
                    sender,
                    receiver,
                    memo: Some(memo),
                    value,
                })
            }
            None => Err(OreoError::InternalRpcError),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTransactionDetail {
    pub account: String,
    pub transaction: TransactionDetail,
}

impl GetTransactionDetail {
    pub fn from_rpc_data(data: GetAccountTransactionRep) -> Result<Self, OreoError> {
        if data.transaction.is_none() || data.notes.is_none() {
            return Err(OreoError::TransactionNotFound);
        }
        let notes = data.notes.unwrap();
        let tx = data.transaction.unwrap();
        let transaction_detail = TransactionDetail::from(notes, tx);
        match transaction_detail {
            Ok(detail) => Ok(Self {
                account: data.account,
                transaction: detail,
            }),
            Err(e) => Err(e),
        }
    }
}
