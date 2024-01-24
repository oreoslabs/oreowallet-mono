use serde::{Deserialize, Serialize};

use crate::{
    error::OreoError,
    rpc_handler::abi::{
        AssetBalanceDelta, CreateAccountOpt, GetAccountTransactionRep, RpcNote,
        TransactionWithNotes,
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
    pub fn from(tx: TransactionWithNotes) -> Result<Self, OreoError> {
        // we handle send/receive transactions only now
        // todo: miner transaction
        let TransactionWithNotes {
            hash,
            fee,
            r#type,
            status,
            block_sequence,
            timestamp,
            asset_balance_deltas,
            notes,
        } = tx;
        let mut notes = notes.unwrap();
        let note = match r#type.as_str() {
            "send" => {
                let mut receiver_candi: Vec<RpcNote> = notes
                    .into_iter()
                    .filter(|note| note.owner != note.sender)
                    .collect();
                // currently, there should be only one receiver in most cases
                receiver_candi.pop()
            }
            "receive" => {
                let mut sender_candi: Vec<RpcNote> = notes
                    .into_iter()
                    .filter(|note| note.owner != note.sender)
                    .collect();
                sender_candi.pop()
            }
            _ => notes.pop(),
        };
        match note {
            Some(RpcNote {
                value,
                memo,
                sender,
                owner,
            }) => Ok(Self {
                hash,
                fee,
                r#type,
                status,
                block_sequence,
                timestamp,
                asset_balance_deltas,
                sender: sender.into(),
                receiver: owner.into(),
                memo: Some(memo.into()),
                value: value.into(),
            }),
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
        if data.transaction.is_none() {
            return Err(OreoError::TransactionNotFound);
        }
        let tx = data.transaction.unwrap();
        let transaction_detail = TransactionDetail::from(tx);
        match transaction_detail {
            Ok(detail) => Ok(Self {
                account: data.account,
                transaction: detail,
            }),
            Err(e) => Err(e),
        }
    }
}
