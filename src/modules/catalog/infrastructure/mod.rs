use axum::Router;

pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

use sqlx::postgres::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub _pool: PgPool,
}

// Placeholder function - TODO: implement router
pub fn create_router(_state: AppState) -> Router {
    Router::new()
    // TODO: Add routes as controllers are implemented
}
