use axum::{
    extract::{self, State},
    Json,
};

use crate::{
    config::ACCOUNT_VERSION,
    db_handler::DBHandler,
    error::OreoError,
    rpc_handler::abi::{
        BroadcastTxRep, BroadcastTxReq, CreateTxRep, CreateTxReq, GetBalancesRep, GetBalancesReq,
        GetTransactionsReq, ImportAccountRep, ImportAccountReq as RpcImportReq, TransactionStatus,
    },
    SharedState,
};

use super::abi::{GetAccountStatusRep, GetAccountStatusReq, GetLatestBlockRep, ImportAccountReq};

pub async fn import_vk_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(import): extract::Json<ImportAccountReq>,
) -> Result<Json<ImportAccountRep>, Json<OreoError>> {
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
        .save_account(public_address.clone(), 0)
        .unwrap();
    let rpc_data = RpcImportReq {
        view_key,
        incoming_view_key,
        outgoing_view_key,
        public_address,
        version: ACCOUNT_VERSION,
        name: account_name.clone(),
        created_at,
    };
    let res = shared.rpc_handler.import_view_only(rpc_data).await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn get_balances_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(get_balance): extract::Json<GetBalancesReq>,
) -> Result<Json<GetBalancesRep>, Json<OreoError>> {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(get_balance.account.clone())
        .unwrap();
    let res = shared
        .rpc_handler
        .get_balance(GetBalancesReq {
            account: account_name,
            confirmations: get_balance.confirmations,
        })
        .await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn get_transactions_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(get_transactions): extract::Json<GetTransactionsReq>,
) -> Result<Json<Vec<TransactionStatus>>, Json<OreoError>> {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(get_transactions.account.clone())
        .unwrap();
    let res = shared
        .rpc_handler
        .get_transactions(GetTransactionsReq {
            account: account_name,
            limit: get_transactions.limit,
        })
        .await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn create_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(create_transaction): extract::Json<CreateTxReq>,
) -> Result<Json<CreateTxRep>, Json<OreoError>> {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(create_transaction.account.clone())
        .unwrap();
    let res = shared
        .rpc_handler
        .create_transaction(CreateTxReq {
            account: account_name,
            outputs: create_transaction.outputs,
            fee: Some(create_transaction.fee.unwrap_or("1".into())),
            expiration_delta: Some(create_transaction.expiration_delta.unwrap_or(30)),
        })
        .await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn broadcast_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(broadcast_transaction): extract::Json<BroadcastTxReq>,
) -> Result<Json<BroadcastTxRep>, Json<OreoError>> {
    let res = shared
        .rpc_handler
        .broadcast_transaction(broadcast_transaction)
        .await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn account_status_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(account): extract::Json<GetAccountStatusReq>,
) -> Result<Json<GetAccountStatusRep>, Json<OreoError>> {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(account.account.clone())
        .unwrap();
    let res = shared
        .rpc_handler
        .get_account_status(GetAccountStatusReq {
            account: account_name,
        })
        .await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}

pub async fn latest_block_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
) -> Result<Json<GetLatestBlockRep>, Json<OreoError>> {
    let res = shared.rpc_handler.get_latest_block().await;
    match res {
        Ok(data) => Ok(Json(data)),
        Err(e) => Err(Json(e)),
    }
}
