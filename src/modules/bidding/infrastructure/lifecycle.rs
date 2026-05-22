use sqlx::postgres::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::modules::bidding::domain::errors::BiddingError;

use super::controllers::finalize_auction_as_system;

const DEFAULT_FINALIZER_INTERVAL_SECS: u64 = 5;
const DEFAULT_FINALIZER_BATCH_SIZE: i64 = 25;

fn parse_env_u64(name: &str, default_value: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

fn parse_env_i64(name: &str, default_value: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

async fn activate_scheduled_auctions(pool: &PgPool) -> Result<u64, BiddingError> {
    // SCHEDULED → ACTIVE once `starts_at` has passed. Catalog creates the
    // auction in SCHEDULED state when starts_at is in the future; this pass
    // promotes it once the start time arrives.
    let affected = sqlx::query(
        r#"
        UPDATE auctions
        SET status = 'ACTIVE'::auction_status
        WHERE status = 'SCHEDULED'::auction_status
          AND starts_at <= CURRENT_TIMESTAMP
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected)
}

async fn close_ended_auctions(pool: &PgPool) -> Result<u64, BiddingError> {
    // ACTIVE/EXTENDED → CLOSED once `ends_at` has passed. CLOSED is the
    // observable intermediate state from the WBS lifecycle
    // (DRAFT → ACTIVE → EXTENDED → CLOSED → WON/UNSOLD); finalize then
    // resolves CLOSED to WON or UNSOLD based on the reserve outcome.
    let affected = sqlx::query(
        r#"
        UPDATE auctions
        SET status = 'CLOSED'::auction_status
        WHERE status IN ('ACTIVE'::auction_status, 'EXTENDED'::auction_status)
          AND ends_at <= CURRENT_TIMESTAMP
        "#,
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(affected)
}

async fn run_auto_finalize_pass(pool: &PgPool, batch_size: i64) -> Result<usize, BiddingError> {
    match activate_scheduled_auctions(pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(activated = n, "scheduled auctions activated"),
        Err(err) => tracing::error!(error = %err, "activate-scheduled pass error"),
    }

    match close_ended_auctions(pool).await {
        Ok(0) => {}
        Ok(n) => tracing::info!(closed = n, "ended auctions marked CLOSED"),
        Err(err) => tracing::error!(error = %err, "close-ended pass error"),
    }

    let auction_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM auctions
        WHERE status = 'CLOSED'::auction_status
        ORDER BY ends_at ASC
        LIMIT $1
        "#,
    )
    .bind(batch_size)
    .fetch_all(pool)
    .await?;

    let mut finalized_count: usize = 0;
    for auction_id in auction_ids {
        match finalize_auction_as_system(pool, auction_id).await {
            Ok(result) => {
                finalized_count += 1;
                tracing::info!(
                    auction_id = %auction_id,
                    status = %result.status,
                    "auto-finalized auction"
                );
            }
            Err(BiddingError::ValidationError(_)) => {
                // Auction time may have shifted due to anti-sniping while this pass is running.
                tracing::debug!(auction_id = %auction_id, "skipped auto-finalize due to timing guard");
            }
            Err(BiddingError::AuctionNotActive) => {
                // Another process may have finalized or changed status concurrently.
                tracing::debug!(auction_id = %auction_id, "skipped auto-finalize because auction is no longer active");
            }
            Err(err) => {
                tracing::error!(auction_id = %auction_id, error = %err, "auto-finalize failed");
            }
        }
    }

    Ok(finalized_count)
}

pub fn spawn_auto_finalize_worker(pool: PgPool) {
    let interval_secs = parse_env_u64(
        "APP_BIDDING_FINALIZER_INTERVAL_SECS",
        DEFAULT_FINALIZER_INTERVAL_SECS,
    );
    let batch_size = parse_env_i64(
        "APP_BIDDING_FINALIZER_BATCH_SIZE",
        DEFAULT_FINALIZER_BATCH_SIZE,
    );

    tracing::info!(
        interval_secs,
        batch_size,
        "starting bidding auto-finalizer worker"
    );

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            ticker.tick().await;
            match run_auto_finalize_pass(&pool, batch_size).await {
                Ok(processed) if processed > 0 => {
                    tracing::info!(processed, "bidding auto-finalizer pass completed");
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(error = %err, "bidding auto-finalizer pass error");
                }
            }
        }
    });
}
