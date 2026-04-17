mod infrastructure;
mod modules;
mod shared;

use axum::serve;
use tokio::net::TcpListener;

use infrastructure::config::AppConfig;
use infrastructure::database::create_pool;
use infrastructure::logger::init_tracer;
use modules::catalog::infrastructure::{create_router, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer();

    let config = AppConfig::new().expect("Failed to load configuration from .env file");
    let pool = create_pool(&config.database_url).await?;

    let state = AppState::new(pool);
    let router = create_router(state);

    let address = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(&address).await?;

    tracing::info!("Starting server on {}", address);

    serve(listener, router).await?;

    Ok(())
}
