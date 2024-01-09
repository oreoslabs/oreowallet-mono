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
