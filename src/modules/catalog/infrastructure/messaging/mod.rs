use futures::StreamExt;
use lapin::{options::*, types::FieldTable, Connection, ConnectionProperties, ExchangeKind};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::catalog::domain::traits::ListingIntegrationPort;
use crate::modules::catalog::infrastructure::repositories::PostgresListingIntegrationRepository;

const EXCHANGE: &str = "bid.events";
const QUEUE: &str = "catalog.bid_updates";
const ROUTING_KEY: &str = "bid.placed";
const CONSUMER_TAG: &str = "catalog_bid_consumer";

#[derive(Debug, Deserialize)]
struct BidPlacedEvent {
    listing_id: Uuid,
    new_price: i64,
}

pub fn spawn_bid_event_consumer(pool: PgPool, amqp_url: Option<String>) {
    let Some(url) = amqp_url else {
        tracing::warn!("APP_AMQP_URL not set — bid event consumer disabled");
        return;
    };

    tokio::spawn(async move {
        loop {
            match run_consumer(&pool, &url).await {
                Ok(_) => {
                    tracing::info!("Bid event consumer stopped cleanly");
                    break;
                }
                Err(e) => {
                    tracing::error!("Bid event consumer error: {e}, retrying in 5s");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
            }
        }
    });
}

async fn run_consumer(
    pool: &PgPool,
    amqp_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let conn = Connection::connect(amqp_url, ConnectionProperties::default()).await?;
    tracing::info!("Bid event consumer connected to RabbitMQ");

    let channel = conn.create_channel().await?;

    channel
        .exchange_declare(
            EXCHANGE,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_declare(
            QUEUE,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            QUEUE,
            EXCHANGE,
            ROUTING_KEY,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let mut consumer = channel
        .basic_consume(
            QUEUE,
            CONSUMER_TAG,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    let repo = PostgresListingIntegrationRepository::new(pool.clone());

    tracing::info!("Bid event consumer listening on queue '{QUEUE}'");

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;

        match serde_json::from_slice::<BidPlacedEvent>(&delivery.data) {
            Ok(event) => {
                let price_ok = repo
                    .update_current_price(event.listing_id, event.new_price)
                    .await
                    .is_ok();
                let count_ok = repo
                    .increment_bid_count(event.listing_id)
                    .await
                    .is_ok();

                if price_ok && count_ok {
                    tracing::info!(
                        "Updated listing {} → price={}",
                        event.listing_id,
                        event.new_price
                    );
                    delivery.ack(BasicAckOptions::default()).await?;
                } else {
                    tracing::error!(
                        "Failed to update listing {}, nacking for retry",
                        event.listing_id
                    );
                    delivery
                        .nack(BasicNackOptions {
                            requeue: true,
                            ..Default::default()
                        })
                        .await?;
                }
            }
            Err(e) => {
                tracing::warn!("Invalid bid event payload: {e}, discarding");
                delivery.ack(BasicAckOptions::default()).await?;
            }
        }
    }

    Ok(())
}
