mod infrastructure;
mod modules;
mod shared;

use axum::middleware;
use axum::serve;
use tokio::net::TcpListener;

use infrastructure::amqp::AmqpPublisher;
use infrastructure::config::AppConfig;
use infrastructure::database::create_pool;
use infrastructure::database::migrations::run_pending_migrations;
use infrastructure::logger::init_tracer;
use infrastructure::logger::request_trace_middleware;
use modules::bidding::create_router as create_bidding_router;
use modules::bidding::infrastructure::lifecycle::spawn_auto_finalize_worker;
use modules::bidding::infrastructure::AppState as BiddingAppState;
use modules::catalog::infrastructure::{create_router as create_catalog_router, AppState};
use modules::order::{
    create_router as create_order_router,
    infrastructure::{create_runtime_app_state_with_auth, lifecycle::spawn_bidding_event_bridge},
};
use modules::wallet::infrastructure::create_router as create_wallet_router;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer();

    let config = AppConfig::new().expect("Failed to load configuration from .env file");
    let pool = create_pool(&config.database_url).await?;
    if config.auto_migrate_on_startup {
        tracing::info!("APP_AUTO_MIGRATE_ON_STARTUP=true, running pending migrations");
        run_pending_migrations(&pool).await?;
    }

    let amqp = match &config.amqp_url {
        Some(url) => match AmqpPublisher::connect(url).await {
            Ok(publisher) => {
                tracing::info!("AMQP publisher connected");
                Some(publisher)
            }
            Err(e) => {
                tracing::warn!(error = %e, "AMQP connection failed — running without realtime push");
                None
            }
        },
        None => {
            tracing::info!("APP_AMQP_URL not set — AMQP publisher disabled");
            None
        }
    };

    let catalog_state = AppState::new(
        pool.clone(),
        config.auth_base_url.clone(),
        config.storage.clone(),
    )
    .await?;
    let bidding_state =
        BiddingAppState::new(pool.clone(), config.auth_base_url.clone(), amqp.clone());
    let order_state =
        create_runtime_app_state_with_auth(pool.clone(), config.auth_base_url.clone());
    spawn_auto_finalize_worker(pool.clone(), amqp);
    spawn_bidding_event_bridge(
        pool.clone(),
        order_state.notification_repo.clone(),
        config.amqp_url.clone(),
    );

    let router = create_catalog_router(catalog_state)
        .merge(create_bidding_router(bidding_state))
        .merge(create_wallet_router(
            pool.clone(),
            config.auth_base_url.clone(),
        ))
        .merge(create_order_router(order_state))
        .layer(middleware::from_fn(request_trace_middleware));

    let address = format!("{}:{}", config.server_host, config.server_port);
    let listener = TcpListener::bind(&address).await?;

    tracing::info!("Starting server on {}", address);

    serve(listener, router).await?;

    Ok(())
}
