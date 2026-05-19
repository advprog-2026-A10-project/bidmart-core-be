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

fn public_wallet_routes() -> Router<WalletAppState> {
    Router::new()
        .route("/wallet", get(get_balance))
        .route("/wallet/topup", post(topup))
        .route("/wallet/withdraw", post(withdraw))
        .route("/wallet/transactions", get(get_transactions))
        .route(
            "/wallet/transactions/:transactionId",
            get(get_transaction_by_id),
        )
}

fn internal_wallet_routes() -> Router<WalletAppState> {
    Router::new()
        .route("/internal/wallet/holds", post(internal_hold))
        .route("/internal/wallet/release", post(internal_release))
        .route("/internal/wallet/payment", post(internal_payment))
}

pub fn create_router(pool: PgPool, auth_base_url: String) -> Router {
    let repo = Arc::new(PostgresWalletRepository::new(Arc::new(pool)));
    let use_cases = Arc::new(WalletUseCases::new(repo));

    let state = WalletAppState {
        use_cases,
        auth_base_url,
        auth_http_client: reqwest::Client::new(),
    };

    // Keep `/api/core/v1` as compatibility alias while standardizing under `/api/v1`.
    Router::new()
        .nest(
            "/api/v1",
            public_wallet_routes().merge(internal_wallet_routes()),
        )
        .nest(
            "/api/core/v1",
            public_wallet_routes().merge(internal_wallet_routes()),
        )
        .with_state(state)
}
