use serde::{Deserialize, Serialize};

use crate::{
    constants::{MAINNET_GENESIS_HASH, MAINNET_GENESIS_SEQUENCE},
    db_handler::{address_to_name, Account},
    error::OreoError,
    rpc_handler::abi::{
        AssetBalanceDelta, CreateAccountOpt, GetAccountTransactionRep, RpcNote,
        TransactionWithNotes,
    },
};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportAccountReq {
    pub view_key: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub public_address: String,
    pub created_at: Option<CreateAccountOpt>,
}

impl ImportAccountReq {
    pub fn to_account(&self) -> Account {
        let (create_head, create_hash) = match &self.created_at {
            Some(creat) => (Some(creat.sequence as i64), Some(creat.hash.clone())),
            None => (None, None),
        };
        Account {
            address: self.public_address.clone(),
            name: address_to_name(&self.public_address),
            create_head,
            create_hash: create_hash.clone(),
            head: create_head.unwrap_or(MAINNET_GENESIS_SEQUENCE),
            hash: create_hash.unwrap_or(MAINNET_GENESIS_HASH.to_string()),
            in_vk: self.incoming_view_key.clone(),
            out_vk: self.outgoing_view_key.clone(),
            vk: self.view_key.clone(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportAccountRep {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveAccountReq {
    pub account: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveAccountRep {
    pub removed: bool,
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
        let note = match notes
            .iter()
            .filter(|asset| asset.owner != asset.sender)
            .collect::<Vec<&RpcNote>>()
            .len()
        {
            0 => notes.pop(),
            _ => notes
                .into_iter()
                .filter(|asset| asset.owner != asset.sender)
                .collect::<Vec<RpcNote>>()
                .pop(),
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
