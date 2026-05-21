pub mod application;
pub mod domain;
pub mod infrastructure;

use self::infrastructure::{controllers, middleware, AppState};
use axum::middleware::from_fn;
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(controllers::orders::router())
        .merge(controllers::notifications::router())
        .merge(controllers::events::router())
        // Per-module request tracer (scoped under `core_be.order.request`).
        .layer(from_fn(middleware::request_trace))
        .with_state(state)
}
