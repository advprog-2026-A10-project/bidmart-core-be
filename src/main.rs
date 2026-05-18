mod infrastructure;
mod modules;
mod shared;

use axum::serve;
use tokio::net::TcpListener;

use infrastructure::config::AppConfig;
use infrastructure::database::create_pool;
use infrastructure::logger::init_tracer;
use modules::example::infrastructure::create_router;
use modules::example::infrastructure::AppState;
use modules::wallet::infrastructure::create_router as create_wallet_router;
use modules::order::{create_router, infrastructure::create_app_state};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer();

    let config = AppConfig::new().expect("Failed to load configuration from .env file");

    let pool = create_pool(&config.database_url).await?;

    let app_state = AppState { _pool: _pool.clone() };

    let router = create_router(app_state).merge(create_wallet_router(_pool));

    let address = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(&address).await?;

    tracing::info!("Starting server on {}", address);

    serve(listener, router).await?;

    Ok(())
}
