use std::time::Duration;

use futures_util::StreamExt;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    Connection, ConnectionProperties, ExchangeKind,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::order::application::dto::PublishEventDto;
use crate::modules::order::application::use_cases::PublishEventUseCase;
use crate::modules::order::domain::entities::{
    NotificationChannel, NotificationEventPayload, NotificationType,
};

use super::NotificationRepositoryHandle;

const AMQP_EXCHANGE: &str = "bidmart.auction";
const AMQP_BIND_PATTERN: &str = "auction.*";
const AMQP_CONSUMER_TAG: &str = "core-order-event-bridge";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BidPlacedEvent {
    auction_id: Uuid,
    current_price: i64,
    bid_count: i32,
    extended: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuctionFinalizedEvent {
    auction_id: Uuid,
    status: String,
    winner_name: Option<String>,
    final_price: Option<i64>,
}

#[derive(Debug)]
struct AuctionSnapshot {
    seller_id: Uuid,
    title: String,
}

fn format_rupiah(amount: i64) -> String {
    let mut digits = amount.abs().to_string().chars().rev().collect::<Vec<_>>();
    let mut grouped = String::new();

    for (index, digit) in digits.drain(..).enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push('.');
        }
        grouped.push(digit);
    }

    let mut formatted = grouped.chars().rev().collect::<String>();
    if amount < 0 {
        formatted = format!("-{formatted}");
    }

    format!("Rp {formatted}")
}

fn parse_env_bool(name: &str, default_value: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" | "on" => Some(true),
            "0" | "false" | "no" | "off" => Some(false),
            _ => None,
        })
        .unwrap_or(default_value)
}

fn reconnect_delay_ms(attempt: u32) -> u64 {
    let exponential = 1_000u64.saturating_mul(2u64.saturating_pow(attempt.min(6)));
    exponential.min(30_000)
}

async fn find_auction_snapshot(
    pool: &PgPool,
    auction_id: Uuid,
) -> Result<Option<AuctionSnapshot>, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, String)>(
        r#"
        SELECT seller_id, title
        FROM auctions
        WHERE id = $1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|(seller_id, title)| AuctionSnapshot { seller_id, title }))
}

async fn find_latest_order_id(
    pool: &PgPool,
    auction_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT id
        FROM orders
        WHERE auction_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(pool)
    .await
}

async fn publish_notification(
    notification_repo: &NotificationRepositoryHandle,
    user_id: Uuid,
    order_id: Option<Uuid>,
    notification_type: NotificationType,
    title: String,
    body: String,
) {
    let payload = NotificationEventPayload {
        order_id,
        notification_type,
        title,
        body,
        channel: NotificationChannel::Inbox,
        metadata: Some(serde_json::json!({
            "userId": user_id.to_string(),
        })),
    };

    if let Err(error) = PublishEventUseCase::new(notification_repo.clone())
        .execute(PublishEventDto { payload })
        .await
    {
        tracing::warn!(?error, %user_id, "order event bridge failed to publish notification");
    }
}

async fn handle_bid_placed(
    pool: &PgPool,
    notification_repo: &NotificationRepositoryHandle,
    payload: BidPlacedEvent,
) {
    let snapshot = match find_auction_snapshot(pool, payload.auction_id).await {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => {
            tracing::debug!(auction_id = %payload.auction_id, "skip bid_placed bridge: auction not found");
            return;
        }
        Err(error) => {
            tracing::warn!(?error, auction_id = %payload.auction_id, "skip bid_placed bridge: lookup failed");
            return;
        }
    };

    let title = format!("BidPlaced · {}", snapshot.title);
    let body = if payload.extended.unwrap_or(false) {
        format!(
            "New bid reached {} ({} bids) and triggered anti-sniping extension.",
            format_rupiah(payload.current_price),
            payload.bid_count,
        )
    } else {
        format!(
            "New bid reached {} with total {} bids.",
            format_rupiah(payload.current_price),
            payload.bid_count,
        )
    };

    publish_notification(
        notification_repo,
        snapshot.seller_id,
        None,
        NotificationType::BidPlaced,
        title,
        body,
    )
    .await;
}

async fn handle_auction_finalized(
    pool: &PgPool,
    notification_repo: &NotificationRepositoryHandle,
    payload: AuctionFinalizedEvent,
) {
    let snapshot = match find_auction_snapshot(pool, payload.auction_id).await {
        Ok(Some(snapshot)) => snapshot,
        Ok(None) => {
            tracing::debug!(auction_id = %payload.auction_id, "skip auction.finalized bridge: auction not found");
            return;
        }
        Err(error) => {
            tracing::warn!(?error, auction_id = %payload.auction_id, "skip auction.finalized bridge: lookup failed");
            return;
        }
    };

    let order_id = match find_latest_order_id(pool, payload.auction_id).await {
        Ok(value) => value,
        Err(error) => {
            tracing::warn!(?error, auction_id = %payload.auction_id, "could not resolve order id for finalized event");
            None
        }
    };

    let final_price = payload
        .final_price
        .map(format_rupiah)
        .unwrap_or_else(|| "N/A".to_string());

    let normalized_status = payload.status.to_ascii_uppercase();
    let body = if normalized_status == "WON" {
        let winner = payload
            .winner_name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "the winner".to_string());
        format!(
            "Auction closed with winner {} at {}. Please continue fulfillment flow.",
            winner, final_price
        )
    } else {
        format!(
            "Auction finalized with status {} and final price {}.",
            normalized_status, final_price
        )
    };

    publish_notification(
        notification_repo,
        snapshot.seller_id,
        order_id,
        NotificationType::WinnerDetermined,
        format!("WinnerDetermined · {}", snapshot.title),
        body,
    )
    .await;
}

async fn handle_auction_event(
    pool: &PgPool,
    notification_repo: &NotificationRepositoryHandle,
    routing_key: &str,
    body: &[u8],
) {
    match routing_key {
        "auction.bid_placed" => match serde_json::from_slice::<BidPlacedEvent>(body) {
            Ok(payload) => handle_bid_placed(pool, notification_repo, payload).await,
            Err(error) => tracing::warn!(
                ?error,
                routing_key,
                "order event bridge ignored malformed auction.bid_placed payload"
            ),
        },
        "auction.finalized" => match serde_json::from_slice::<AuctionFinalizedEvent>(body) {
            Ok(payload) => handle_auction_finalized(pool, notification_repo, payload).await,
            Err(error) => tracing::warn!(
                ?error,
                routing_key,
                "order event bridge ignored malformed auction.finalized payload"
            ),
        },
        _ => {
            tracing::debug!(
                routing_key,
                "order event bridge ignored unsupported routing key"
            );
        }
    }
}

async fn run_consumer_once(
    pool: &PgPool,
    notification_repo: &NotificationRepositoryHandle,
    amqp_url: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let connection = Connection::connect(amqp_url, ConnectionProperties::default()).await?;
    let channel = connection.create_channel().await?;

    channel
        .exchange_declare(
            AMQP_EXCHANGE,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    let queue_name = format!("core-order-events-{}", Uuid::new_v4());
    let queue = channel
        .queue_declare(
            &queue_name,
            QueueDeclareOptions {
                durable: false,
                exclusive: true,
                auto_delete: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .queue_bind(
            queue.name().as_str(),
            AMQP_EXCHANGE,
            AMQP_BIND_PATTERN,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await?;

    tracing::info!(
        exchange = AMQP_EXCHANGE,
        queue = %queue.name(),
        "order event bridge AMQP consumer ready"
    );

    let mut consumer = channel
        .basic_consume(
            queue.name().as_str(),
            AMQP_CONSUMER_TAG,
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.next().await {
        let delivery = delivery?;
        let routing_key = delivery.routing_key.clone();

        handle_auction_event(
            pool,
            notification_repo,
            routing_key.as_str(),
            &delivery.data,
        )
        .await;

        if let Err(error) = delivery.ack(BasicAckOptions::default()).await {
            tracing::warn!(?error, "order event bridge ack failed");
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::UnexpectedEof,
        "order event bridge AMQP stream ended",
    )
    .into())
}

pub fn spawn_bidding_event_bridge(
    pool: PgPool,
    notification_repo: NotificationRepositoryHandle,
    amqp_url: Option<String>,
) {
    if !parse_env_bool("APP_ORDER_EVENT_BRIDGE_ENABLED", true) {
        tracing::info!("order event bridge disabled by APP_ORDER_EVENT_BRIDGE_ENABLED=false");
        return;
    }

    let Some(amqp_url) = amqp_url.filter(|value| !value.trim().is_empty()) else {
        tracing::info!("order event bridge skipped: APP_AMQP_URL is not configured");
        return;
    };

    tracing::info!("starting order event bridge worker");

    tokio::spawn(async move {
        let mut attempt = 0u32;

        loop {
            match run_consumer_once(&pool, &notification_repo, &amqp_url).await {
                Ok(()) => {
                    attempt = 0;
                }
                Err(error) => {
                    attempt = attempt.saturating_add(1);
                    let delay_ms = reconnect_delay_ms(attempt);
                    tracing::warn!(
                        ?error,
                        attempt,
                        delay_ms,
                        "order event bridge consumer failed, reconnecting"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    });
}
