use axum::{
    extract::{self, State},
    response::IntoResponse,
};

use crate::{
    constants::ACCOUNT_VERSION,
    db_handler::DBHandler,
    error::OreoError,
    rpc_handler::abi::{
        BroadcastTxReq, CreateTxReq, GetAccountTransactionReq, GetBalancesRep, GetBalancesReq,
        GetTransactionsReq, ImportAccountReq as RpcImportReq, OutPut, RpcResponse,
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
    shared
        .rpc_handler
        .import_view_only(rpc_data)
        .await
        .into_response()
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
    let resp = shared
        .rpc_handler
        .get_balance(GetBalancesReq {
            account: account_name.unwrap(),
            confirmations: Some(get_balance.confirmations.unwrap_or(10)),
        })
        .await;
    match resp {
        Ok(res) => {
            let response = RpcResponse {
                status: 200,
                data: GetBalancesRep::verified_asset(res.data),
            };
            response.into_response()
        }
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
    shared
        .rpc_handler
        .get_transactions(GetTransactionsReq {
            account: account_name.unwrap(),
            limit: Some(get_transactions.limit.unwrap_or(6)),
            reverse: Some(true),
        })
        .await
        .into_response()
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
    let outputs: Vec<OutPut> = create_transaction
        .outputs
        .unwrap_or(vec![])
        .iter()
        .map(|output| OutPut::from(output.clone()))
        .collect();
    let mut mints = vec![];
    for item in create_transaction.mints.unwrap_or(vec![]).into_iter() {
        if item.asset_id.is_none() && item.name.is_none() {
            return OreoError::BadMintRequest.into_response();
        } else {
            mints.push(item);
        }
    }
    let burns = create_transaction.burns.unwrap_or(vec![]);
    shared
        .rpc_handler
        .create_transaction(CreateTxReq {
            account: account_name.unwrap(),
            outputs: Some(outputs),
            fee: Some(create_transaction.fee.unwrap_or("1".into())),
            expiration_delta: Some(create_transaction.expiration_delta.unwrap_or(30)),
            mints: Some(mints),
            burns: Some(burns),
        })
        .await
        .into_response()
}

pub async fn broadcast_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(broadcast_transaction): extract::Json<BroadcastTxReq>,
) -> impl IntoResponse {
    shared
        .rpc_handler
        .broadcast_transaction(broadcast_transaction)
        .await
        .into_response()
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
    shared
        .rpc_handler
        .get_account_status(GetAccountStatusReq {
            account: account_name.unwrap(),
        })
        .await
        .into_response()
}

pub async fn latest_block_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
) -> impl IntoResponse {
    shared.rpc_handler.get_latest_block().await.into_response()
}

pub async fn account_transaction_handler<T: DBHandler>(
    State(shared): State<SharedState<T>>,
    extract::Json(account): extract::Json<GetAccountTransactionReq>,
) -> impl IntoResponse {
    let account_name = shared
        .db_handler
        .lock()
        .await
        .get_account(account.account.clone());
    if let Err(e) = account_name {
        return e.into_response();
    }
    shared
        .rpc_handler
        .get_account_transaction(GetAccountTransactionReq {
            account: account_name.unwrap(),
            hash: account.hash,
        })
        .await
        .into_response()
}
