use axum::{
    extract::{self, State},
    response::IntoResponse,
};

use crate::{
    config::ACCOUNT_VERSION,
    db_handler::DBHandler,
    rpc_handler::abi::{
        BroadcastTxReq, CreateTxReq, GetBalancesReq, GetTransactionsReq,
        ImportAccountReq as RpcImportReq,
    },
    SharedState,
};

use super::abi::{GetAccountStatusReq, ImportAccountReq};

pub async fn import_vk_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(import): extract::Json<ImportAccountReq>,
) -> impl IntoResponse {
    let ImportAccountReq {
        view_key,
        incoming_view_key,
        outgoing_view_key,
        public_address,
        created_at,
    } = import;
    let account_name = shared
        .db_handler
        .lock()
        .await
        .save_account(public_address.clone(), 0);
    if let Err(e) = account_name {
        return e.into_response();
    }
    let rpc_data = RpcImportReq {
        view_key,
        incoming_view_key,
        outgoing_view_key,
        public_address,
        version: ACCOUNT_VERSION,
        name: account_name.unwrap(),
        created_at,
    };
    let res = shared.rpc_handler.import_view_only(rpc_data).await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_balances_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(get_balance): extract::Json<GetBalancesReq>,
) -> impl IntoResponse {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(get_balance.account.clone());
    if let Err(e) = account_name {
        return e.into_response();
    }
    let res = shared
        .rpc_handler
        .get_balance(GetBalancesReq {
            account: account_name.unwrap(),
            confirmations: get_balance.confirmations,
        })
        .await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn get_transactions_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(get_transactions): extract::Json<GetTransactionsReq>,
) -> impl IntoResponse {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(get_transactions.account.clone());
    if let Err(e) = account_name {
        return e.into_response();
    }
    let res = shared
        .rpc_handler
        .get_transactions(GetTransactionsReq {
            account: account_name.unwrap(),
            limit: get_transactions.limit,
        })
        .await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn create_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(create_transaction): extract::Json<CreateTxReq>,
) -> impl IntoResponse {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(create_transaction.account.clone());
    if let Err(e) = account_name {
        return e.into_response();
    }
    let res = shared
        .rpc_handler
        .create_transaction(CreateTxReq {
            account: account_name.unwrap(),
            outputs: create_transaction.outputs,
            fee: Some(create_transaction.fee.unwrap_or("1".into())),
            expiration_delta: Some(create_transaction.expiration_delta.unwrap_or(30)),
        })
        .await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn broadcast_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(broadcast_transaction): extract::Json<BroadcastTxReq>,
) -> impl IntoResponse {
    let res = shared
        .rpc_handler
        .broadcast_transaction(broadcast_transaction)
        .await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn account_status_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(account): extract::Json<GetAccountStatusReq>,
) -> impl IntoResponse {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(account.account.clone());
    if let Err(e) = account_name {
        return e.into_response();
    }
    let res = shared
        .rpc_handler
        .get_account_status(GetAccountStatusReq {
            account: account_name.unwrap(),
        })
        .await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}

pub async fn latest_block_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
) -> impl IntoResponse {
    let res = shared.rpc_handler.get_latest_block().await;
    match res {
        Ok(data) => data.into_response(),
        Err(e) => e.into_response(),
    }
}
