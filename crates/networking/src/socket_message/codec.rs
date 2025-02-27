use anyhow::Result;
use bytes::{BufMut, BytesMut};
use db_handler::{Account, DBTransaction};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tokio_util::codec::{Decoder, Encoder};
use uuid::Uuid;

use crate::decryption_message::ScanRequest;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct RegisterWorker {
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SingleRequest {
    pub serialized_note: Vec<String>,
    pub tx_hash: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DRequest {
    pub id: String,
    pub address: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub decrypt_for_spender: bool,
    pub data: Vec<SingleRequest>,
}

impl DRequest {
    pub fn new(account: &Account, transaction: DBTransaction) -> Self {
        let tx_hash = transaction.hash;
        let serializeds = transaction.serialized_notes;
        let data = SingleRequest {
            tx_hash,
            serialized_note: serializeds,
        };
        Self {
            id: Uuid::new_v4().to_string(),
            address: account.address.clone(),
            incoming_view_key: account.in_vk.clone(),
            outgoing_view_key: account.out_vk.clone(),
            decrypt_for_spender: true,
            data: vec![data],
        }
    }

    pub fn from_transactions(account: &ScanRequest, transactions: Vec<DBTransaction>) -> Self {
        let data = transactions
            .into_iter()
            .map(|tx| SingleRequest {
                tx_hash: tx.hash.to_string(),
                serialized_note: tx.serialized_notes,
            })
            .collect();
        Self {
            id: Uuid::new_v4().to_string(),
            address: account.address.clone(),
            incoming_view_key: account.in_vk.clone(),
            outgoing_view_key: account.out_vk.clone(),
            decrypt_for_spender: true,
            data,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct DResponse {
    pub address: String,
    pub id: String,
    pub data: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum DMessage {
    RegisterWorker(RegisterWorker),
    DRequest(DRequest),
    DResponse(DResponse),
}

#[derive(Default)]
pub struct DMessageCodec {
    cursor: usize,
}

impl Encoder<DMessage> for DMessageCodec {
    type Error = anyhow::Error;
    fn encode(&mut self, message: DMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let json_string = serde_json::to_string(&message).unwrap();
        dst.writer().write_all(json_string.as_bytes())?;
        dst.writer().write_all("\n".as_bytes())?;
        Ok(())
    }
}

impl Decoder for DMessageCodec {
    type Error = anyhow::Error;
    type Item = DMessage;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let mut i = self.cursor;
        while i < src.len() {
            if src[i] == 10u8 {
                self.cursor = 0;
                let mut data = src.split_to(i + 1);
                unsafe {
                    data.set_len(i);
                }
                src.reserve(100);
                let message = serde_json::from_slice(&data[..])?;
                return Ok(Some(message));
            }
            i += 1;
        }
        self.cursor = i;
        Ok(None)
    }
}
