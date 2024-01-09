use axum::{
    extract::{self, State},
    Json,
};

use crate::{
    config::ACCOUNT_VERSION,
    db_handler::DBHandler,
    rpc_handler::abi::{
        BroadcastTxRep, BroadcastTxReq, CreateTxRep, CreateTxReq, GetBalancesRep, GetBalancesReq,
        GetTransactionsReq, ImportAccountRep, ImportAccountReq as RpcImportReq, TransactionStatus,
    },
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

pub async fn get_balances_handler<T: DBHandler>(
    State(shared): State<Store<T>>,
    extract::Json(get_balance): extract::Json<GetBalancesReq>,
) -> Json<GetBalancesRep> {
    let account_name = shared
        .inner
        .lock()
        .await
        .db_handler
        .get_account(get_balance.account.clone())
        .unwrap();
    let res = shared
        .inner
        .lock()
        .await
        .rpc_handler
        .get_balance(GetBalancesReq {
            account: account_name,
            confirmations: get_balance.confirmations,
        })
        .await
        .unwrap();
    Json(res)
}

pub async fn get_transactions_handler<T: DBHandler>(
    State(shared): State<Store<T>>,
    extract::Json(get_transactions): extract::Json<GetTransactionsReq>,
) -> Json<Vec<TransactionStatus>> {
    let account_name = shared
        .inner
        .lock()
        .await
        .db_handler
        .get_account(get_transactions.account.clone())
        .unwrap();
    let res = shared
        .inner
        .lock()
        .await
        .rpc_handler
        .get_transactions(GetTransactionsReq {
            account: account_name,
            limit: get_transactions.limit,
        })
        .await
        .unwrap();
    Json(res)
}

pub async fn create_transaction_handler<T: DBHandler>(
    State(shared): State<Store<T>>,
    extract::Json(create_transaction): extract::Json<CreateTxReq>,
) -> Json<CreateTxRep> {
    let account_name = shared
        .inner
        .lock()
        .await
        .db_handler
        .get_account(create_transaction.account.clone())
        .unwrap();
    let res = shared
        .inner
        .lock()
        .await
        .rpc_handler
        .create_transaction(CreateTxReq {
            account: account_name,
            outputs: create_transaction.outputs,
            fee: Some(create_transaction.fee.unwrap_or("1".into())),
            expiration_delta: Some(create_transaction.expiration_delta.unwrap_or(30)),
        })
        .await
        .unwrap();
    Json(res)
}

pub async fn broadcast_transaction_handler<T: DBHandler>(
    State(shared): State<Store<T>>,
    extract::Json(broadcast_transaction): extract::Json<BroadcastTxReq>,
) -> Json<BroadcastTxRep> {
    let res = shared
        .inner
        .lock()
        .await
        .rpc_handler
        .broadcast_transaction(broadcast_transaction)
        .await
        .unwrap();
    Json(res)
}
