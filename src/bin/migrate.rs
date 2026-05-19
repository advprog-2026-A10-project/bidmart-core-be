use bidmart_core_be::infrastructure::config::AppConfig;
use bidmart_core_be::infrastructure::database::create_pool;
use bidmart_core_be::infrastructure::database::migrations::run_pending_migrations;
use bidmart_core_be::infrastructure::logger::init_tracer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracer();
    let config = AppConfig::new().expect("Failed to load configuration from .env file");
    let pool = create_pool(&config.database_url).await?;

    run_pending_migrations(&pool).await?;
    tracing::info!("Core database migrations applied successfully");
    Ok(())
}
