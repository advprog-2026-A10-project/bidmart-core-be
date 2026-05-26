pub mod controllers;
pub mod lifecycle;
pub mod middleware;

use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing, Router,
};
use reqwest::Client;
use sqlx::postgres::PgPool;

use crate::infrastructure::amqp::AmqpPublisher;
use controllers::{
    disable_my_proxy_bid, finalize_auction, get_auction_detail, get_auction_history,
    get_my_bid_detail, get_my_proxy_bid, list_my_bids, place_bid, upsert_my_proxy_bid,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_base_url: String,
    pub auth_http_client: Client,
    pub amqp: Option<AmqpPublisher>,
}

impl AppState {
    pub fn new(pool: PgPool, auth_base_url: String, amqp: Option<AmqpPublisher>) -> Self {
        Self {
            pool,
            auth_base_url,
            auth_http_client: Client::new(),
            amqp,
        }
    }
}

pub fn create_router(state: AppState) -> Router {
    let protected_routes = Router::new()
        .route("/auctions/:auction_id", routing::get(get_auction_detail))
        .route(
            "/auctions/:auction_id/history",
            routing::get(get_auction_history),
        )
        .route("/auctions/:auction_id/bids", routing::post(place_bid))
        .route(
            "/auctions/:auction_id/proxy",
            routing::get(get_my_proxy_bid)
                .put(upsert_my_proxy_bid)
                .delete(disable_my_proxy_bid),
        )
        .route(
            "/auctions/:auction_id/finalize",
            routing::post(finalize_auction),
        )
        .route("/me/bids", routing::get(list_my_bids))
        .route("/me/bids/:auction_id", routing::get(get_my_bid_detail))
        .layer(from_fn_with_state(state.clone(), middleware::require_auth));

    Router::new()
        .nest("/api/v1", protected_routes)
        // Per-module request tracer (scoped under `core_be.bidding.request`).
        .layer(from_fn(middleware::request_trace))
        .with_state(state)
}
