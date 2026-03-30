use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::modules::wallet::application::dto::{
    InternalHoldRequest, InternalHoldResponse, TopupRequest, TopupResponse, TransactionListResponse,
    WalletBalanceResponse, WithdrawRequest, WithdrawResponse,
};
use crate::modules::wallet::application::use_cases::WalletUseCases;
use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::infrastructure::middleware::{AuthUser, InternalAuth};

#[derive(Clone)]
pub struct WalletAppState {
    pub use_cases: Arc<WalletUseCases>,
}

pub async fn get_balance(
    State(state): State<WalletAppState>,
    user: AuthUser,
) -> Result<Json<WalletBalanceResponse>, WalletError> {
    let response = state.use_cases.get_balance(user.user_id).await?;
    Ok(Json(response))
}

pub async fn topup(
    State(state): State<WalletAppState>,
    user: AuthUser,
    Json(payload): Json<TopupRequest>,
) -> Result<Json<TopupResponse>, WalletError> {
    let response = state.use_cases.topup(user.user_id, payload).await?;
    Ok(Json(response))
}

pub async fn withdraw(
    State(state): State<WalletAppState>,
    user: AuthUser,
    Json(payload): Json<WithdrawRequest>,
) -> Result<Json<WithdrawResponse>, WalletError> {
    let response = state.use_cases.withdraw(user.user_id, payload).await?;
    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    #[serde(rename = "pageSize")]
    pub page_size: Option<i64>,
}

pub async fn get_transactions(
    State(state): State<WalletAppState>,
    user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> Result<Json<TransactionListResponse>, WalletError> {
    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(50);
    
    let response = state
        .use_cases
        .list_transactions(user.user_id, page, page_size)
        .await?;
        
    Ok(Json(response))
}

pub async fn get_transaction_by_id(
    State(state): State<WalletAppState>,
    user: AuthUser,
    axum::extract::Path(transaction_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<crate::modules::wallet::application::dto::TransactionDto>, WalletError> {
    let response = state.use_cases.get_transaction_by_id(user.user_id, transaction_id).await?;
    Ok(Json(response))
}

pub async fn internal_hold(
    State(state): State<WalletAppState>,
    _auth: InternalAuth,
    Json(payload): Json<InternalHoldRequest>,
) -> Result<Json<InternalHoldResponse>, WalletError> {
    let response = state.use_cases.internal_hold(payload).await?;
    Ok(Json(response))
}
