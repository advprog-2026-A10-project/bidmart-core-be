pub mod application;
pub mod domain;
pub mod infrastructure;

use self::infrastructure::{controllers, AppState};
use axum::Router;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .merge(controllers::orders::router())
        .merge(controllers::notifications::router())
        .merge(controllers::events::router())
        .with_state(state)
}
