mod infrastructure;
mod modules;
mod shared;

use axum::serve;
use tokio::net::TcpListener;

use infrastructure::config::AppConfig;
use infrastructure::database::create_pool;
use infrastructure::logger::init_tracer;
use modules::catalog::infrastructure::{create_router as create_catalog_router, AppState};
use modules::order::{create_router as create_order_router, infrastructure::create_app_state};
use modules::wallet::infrastructure::create_router as create_wallet_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer();

    let config = AppConfig::new().expect("Failed to load configuration from .env file");
    let pool = create_pool(&config.database_url).await?;

    let catalog_state = AppState::new(pool.clone());
    let order_state = create_app_state(pool.clone());

    let router = create_catalog_router(catalog_state)
        .merge(create_wallet_router(pool.clone()))
        .merge(create_order_router(order_state));

    let address = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(&address).await?;

    tracing::info!("Starting server on {}", address);

    serve(listener, router).await?;

    Ok(())
}