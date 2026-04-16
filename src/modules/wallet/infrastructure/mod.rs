pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use sqlx::PgPool;

use crate::modules::wallet::application::use_cases::WalletUseCases;
use crate::modules::wallet::infrastructure::controllers::{
    get_balance, get_transaction_by_id, get_transactions, internal_hold, internal_payment,
    internal_release, topup, withdraw, WalletAppState,
};
use crate::modules::wallet::infrastructure::repositories::PostgresWalletRepository;

pub fn create_router(pool: PgPool) -> Router {
    let repo = Arc::new(PostgresWalletRepository::new(Arc::new(pool)));
    let use_cases = Arc::new(WalletUseCases::new(repo));

    let state = WalletAppState { use_cases };

    Router::new()
        .route("/api/core/v1/wallet", get(get_balance))
        .route("/api/core/v1/wallet/topup", post(topup))
        .route("/api/core/v1/wallet/withdraw", post(withdraw))
        .route("/api/core/v1/wallet/transactions", get(get_transactions))
        .route(
            "/api/core/v1/wallet/transactions/:transactionId",
            get(get_transaction_by_id),
        )
        .route("/api/core/v1/internal/wallet/holds", post(internal_hold))
        .route(
            "/api/core/v1/internal/wallet/release",
            post(internal_release),
        )
        .route(
            "/api/core/v1/internal/wallet/payment",
            post(internal_payment),
        )
        .with_state(state)
}
