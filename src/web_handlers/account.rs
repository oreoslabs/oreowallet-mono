use axum::{
    extract::{self, State},
    Json,
};

use crate::{
    config::ACCOUNT_VERSION,
    db_handler::DBHandler,
    rpc_handler::abi::{ImportAccountRep, ImportAccountReq as RpcImportReq},
    Store,
};

use super::abi::ImportAccountReq;

pub async fn import_vk_handler<T: DBHandler>(
    State(shared): State<Store<T>>,
    extract::Json(import): extract::Json<ImportAccountReq>,
) -> Json<ImportAccountRep> {
    let ImportAccountReq {
        view_key,
        incoming_view_key,
        outgoing_view_key,
        public_address,
    } = import;
    let account_name = shared
        .inner
        .lock()
        .await
        .db_handler
        .save_account(public_address.clone(), 0)
        .unwrap();
    let rpc_data = RpcImportReq {
        view_key,
        incoming_view_key,
        outgoing_view_key,
        public_address,
        version: ACCOUNT_VERSION,
        name: account_name.clone(),
    };
    let res = shared
        .inner
        .lock()
        .await
        .rpc_handler
        .import_view_only(rpc_data)
        .await
        .unwrap();
    Json(res)
}
