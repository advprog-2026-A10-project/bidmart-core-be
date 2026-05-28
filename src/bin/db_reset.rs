use anyhow::{Context, Result};
use bidmart_core_be::infrastructure::config::AppConfig;
use bidmart_core_be::infrastructure::database::create_pool;
use bidmart_core_be::infrastructure::logger::init_tracer;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracer();
    dotenv::dotenv().ok();

    let config = AppConfig::new().context("failed to load app config")?;
    let pool = create_pool(&config.database_url).await?;

    // Keep this explicit to avoid touching sqlx migration metadata tables.
    sqlx::query(
        r#"
        TRUNCATE TABLE
            notifications,
            disputes,
            orders,
            wallet_transactions,
            wallets,
            proxy_bids,
            bids,
            auctions,
            listing_images,
            listings,
            categories
        RESTART IDENTITY CASCADE
        "#,
    )
    .execute(&pool)
    .await
    .context("failed to truncate core application tables")?;

    println!("Core database reset completed.");
    Ok(())
}
