use ironfish_zkp::proofs::Spend;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportAccountReq {
    pub view_key: String,
    pub incoming_view_key: String,
    pub outgoing_view_key: String,
    pub public_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ImportAccountRep {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenerateProofReuqest {
    pub circuits: Vec<u32>,
    // pub spends: Vec<Spend>,
}