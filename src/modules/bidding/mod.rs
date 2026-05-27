pub mod application;
pub mod domain;
pub mod infrastructure;

use axum::Router;
use infrastructure::AppState;

pub fn create_router(state: AppState) -> Router {
    infrastructure::create_router(state)
}
