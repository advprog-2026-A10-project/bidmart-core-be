use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};
use std::cmp::Ordering;
use uuid::Uuid;

use crate::modules::bidding::domain::errors::BiddingError;
use crate::modules::bidding::infrastructure::AppState;
use crate::modules::bidding::infrastructure::middleware::AuthUser;

#[derive(Serialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuctionDetailResponse {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub seller_id: Uuid,
    pub seller_name: String,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub start_price: i64,
    pub current_price: i64,
    pub reserve_price: Option<i64>,
    pub bid_increment: i64,
    pub bid_count: i32,
    pub status: String,
    pub winner_id: Option<Uuid>,
    pub winner_name: Option<String>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub original_ends_at: DateTime<Utc>,
    pub extension_count: i32,
    pub created_at: DateTime<Utc>,
    pub highest_bidder_alias: Option<String>,
    pub my_latest_bid: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BidHistoryEntryResponse {
    pub id: Uuid,
    pub auction_id: Uuid,
    pub bidder_id: Uuid,
    pub bidder_name: String,
    pub amount: i64,
    pub is_proxy: bool,
    pub is_winning: bool,
    pub status: String,
    pub accepted_at: DateTime<Utc>,
    pub received_sequence: i64,
    pub is_my_bid: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionHistoryOrderingResponse {
    pub primary: String,
    pub secondary: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuctionHistoryResponse {
    pub auction: AuctionDetailResponse,
    pub bids: Vec<BidHistoryEntryResponse>,
    pub ordering: AuctionHistoryOrderingResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceBidRequest {
    pub amount: i64,
    pub max_amount: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceBidResponse {
    pub bid_id: Uuid,
    pub auction_id: Uuid,
    pub current_price: i64,
    pub bid_count: i32,
    pub ends_at: DateTime<Utc>,
    pub extended: bool,
    pub extension_count: i32,
    pub minimum_next_bid: i64,
    pub auto_bid_applied: bool,
    pub effective_winner_id: Uuid,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertProxyBidRequest {
    pub max_amount: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyBidResponse {
    pub auction_id: Uuid,
    pub bidder_id: Uuid,
    pub enabled: bool,
    pub max_amount: Option<i64>,
    pub auction_status: String,
    pub current_price: i64,
    pub minimum_proxy_amount: i64,
    pub currently_leading: bool,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisableProxyBidResponse {
    pub auction_id: Uuid,
    pub bidder_id: Uuid,
    pub disabled: bool,
}

#[derive(Deserialize)]
pub struct FinalizeAuctionQuery {
    pub force: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeAuctionResponse {
    pub auction_id: Uuid,
    pub status: String,
    pub winner_id: Option<Uuid>,
    pub winner_name: Option<String>,
    pub final_price: Option<i64>,
    pub order_id: Option<Uuid>,
}

#[derive(Deserialize)]
pub struct MyBidQuery {
    pub status: Option<String>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MyBidItemResponse {
    pub auction_id: Uuid,
    pub listing_id: Uuid,
    pub seller_id: Uuid,
    pub seller_name: String,
    pub title: String,
    pub description: String,
    pub image_url: String,
    pub start_price: i64,
    pub current_price: i64,
    pub reserve_price: Option<i64>,
    pub bid_increment: i64,
    pub bid_count: i32,
    pub auction_status: String,
    pub my_bid_status: String,
    pub winner_id: Option<Uuid>,
    pub ends_at: DateTime<Utc>,
    pub starts_at: DateTime<Utc>,
    pub original_ends_at: DateTime<Utc>,
    pub extension_count: i32,
    pub highest_bidder_alias: Option<String>,
    pub winner_name: Option<String>,
    pub my_latest_bid: i64,
    pub last_bid_at: DateTime<Utc>,
    pub is_reserve_met: bool,
    pub currency: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyBidSummaryResponse {
    pub all: i64,
    pub winning: i64,
    pub outbid: i64,
    pub won: i64,
    pub lost: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyBidListResponse {
    pub user: MyBidUserResponse,
    pub data: Vec<MyBidItemResponse>,
    pub summary: MyBidSummaryResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyBidUserResponse {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyBidTimelineEventResponse {
    pub at: DateTime<Utc>,
    pub r#type: String,
    pub amount: Option<i64>,
    pub note: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MyBidDetailResponse {
    pub auction: AuctionDetailResponse,
    pub my_latest_bid: i64,
    pub my_bid_status: String,
    pub winning_gap: i64,
    pub is_reserve_met: bool,
    pub placed_bid_count: i64,
    pub timeline: Vec<MyBidTimelineEventResponse>,
}

fn classify_my_bid_status(
    auction_status: &str,
    leading_bidder_id: Option<Uuid>,
    winner_id: Option<Uuid>,
    user_id: Uuid,
) -> String {
    match auction_status {
        "ACTIVE" | "EXTENDED" => {
            if leading_bidder_id == Some(user_id) {
                "WINNING".to_string()
            } else {
                "OUTBID".to_string()
            }
        }
        "WON" => {
            if winner_id == Some(user_id) {
                "WON".to_string()
            } else {
                "LOST".to_string()
            }
        }
        _ => "LOST".to_string(),
    }
}

#[derive(Clone)]
struct ProxyCapability {
    bidder_id: Uuid,
    bidder_name: String,
    max_amount: i64,
    priority_at: DateTime<Utc>,
    has_active_proxy: bool,
}

async fn fetch_active_proxy_capabilities(
    tx: &mut Transaction<'_, Postgres>,
    auction_id: Uuid,
) -> Result<Vec<ProxyCapability>, BiddingError> {
    let rows = sqlx::query(
        r#"
        SELECT
            pb.bidder_id,
            pb.max_amount,
            pb.created_at,
            COALESCE(
                (
                    SELECT b.bidder_name
                    FROM bids b
                    WHERE b.auction_id = pb.auction_id
                      AND b.bidder_id = pb.bidder_id
                    ORDER BY b.created_at DESC, b.id DESC
                    LIMIT 1
                ),
                ''
            ) AS bidder_name
        FROM proxy_bids pb
        WHERE pb.auction_id = $1
          AND pb.is_active = true
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(|row| -> Result<ProxyCapability, sqlx::Error> {
            Ok(ProxyCapability {
                bidder_id: row.try_get("bidder_id")?,
                bidder_name: row.try_get("bidder_name")?,
                max_amount: row.try_get("max_amount")?,
                priority_at: row.try_get("created_at")?,
                has_active_proxy: true,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(BiddingError::from)
}

async fn deactivate_proxy_bid(
    tx: &mut Transaction<'_, Postgres>,
    auction_id: Uuid,
    bidder_id: Uuid,
) -> Result<(), BiddingError> {
    sqlx::query(
        r#"
        UPDATE proxy_bids
        SET is_active = false,
            updated_at = CURRENT_TIMESTAMP
        WHERE auction_id = $1
          AND bidder_id = $2
          AND is_active = true
        "#,
    )
    .bind(auction_id)
    .bind(bidder_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn sort_proxy_capabilities(capabilities: &mut [ProxyCapability]) {
    capabilities.sort_by(|left, right| {
        right
            .max_amount
            .cmp(&left.max_amount)
            .then_with(|| left.priority_at.cmp(&right.priority_at))
            .then_with(|| {
                if left.bidder_id == right.bidder_id {
                    Ordering::Equal
                } else {
                    left.bidder_id.as_bytes().cmp(right.bidder_id.as_bytes())
                }
            })
    });
}

fn compute_minimum_proxy_amount(
    current_price: i64,
    bid_increment: i64,
    currently_leading: bool,
    leading_amount: Option<i64>,
) -> i64 {
    if currently_leading {
        leading_amount.unwrap_or(current_price)
    } else {
        current_price.saturating_add(bid_increment)
    }
}

async fn hold_wallet_for_bid(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    auction_id: Uuid,
    amount_to_hold: i64,
) -> Result<(), BiddingError> {
    if amount_to_hold <= 0 {
        return Ok(());
    }

    let wallet_row = sqlx::query(
        r#"
        SELECT user_id, balance, held_balance
        FROM wallets
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::InsufficientBalance)?;

    let balance: i64 = wallet_row.try_get("balance")?;
    let held_balance: i64 = wallet_row.try_get("held_balance")?;
    let available = balance.saturating_sub(held_balance);
    if available < amount_to_hold {
        return Err(BiddingError::InsufficientBalance);
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET held_balance = held_balance + $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND (held_balance + $2) <= balance
        RETURNING balance, held_balance
        "#,
    )
    .bind(user_id)
    .bind(amount_to_hold)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (
            wallet_id,
            type,
            status,
            amount,
            balance_after,
            reference_id,
            reference_type,
            description,
            completed_at
        )
        VALUES (
            $1,
            'BID_HOLD'::transaction_type,
            'COMPLETED'::transaction_status,
            $2,
            $3,
            $4,
            'auction'::reference_type,
            $5,
            CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(user_id)
    .bind(amount_to_hold)
    .bind(balance_after)
    .bind(auction_id)
    .bind(format!("Bid hold for auction {}", auction_id))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn create_notification(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    notification_type: &'static str,
    title: &str,
    message: &str,
    reference_id: Uuid,
) -> Result<(), BiddingError> {
    sqlx::query(
        r#"
        INSERT INTO notifications (
            user_id,
            type,
            title,
            message,
            reference_id,
            reference_type
        )
        VALUES (
            $1,
            $2::notification_type,
            $3,
            $4,
            $5,
            'auction'::reference_type
        )
        "#,
    )
    .bind(user_id)
    .bind(notification_type)
    .bind(title)
    .bind(message)
    .bind(reference_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn release_wallet_for_outbid(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    auction_id: Uuid,
    amount_to_release: i64,
) -> Result<(), BiddingError> {
    if amount_to_release <= 0 {
        return Ok(());
    }

    let wallet_row = sqlx::query(
        r#"
        SELECT user_id, balance, held_balance
        FROM wallets
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::InsufficientBalance)?;

    let held_balance: i64 = wallet_row.try_get("held_balance")?;
    if held_balance < amount_to_release {
        return Err(BiddingError::InsufficientBalance);
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET held_balance = held_balance - $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND (held_balance - $2) >= 0
        RETURNING balance, held_balance
        "#,
    )
    .bind(user_id)
    .bind(amount_to_release)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (
            wallet_id,
            type,
            status,
            amount,
            balance_after,
            reference_id,
            reference_type,
            description,
            completed_at
        )
        VALUES (
            $1,
            'BID_RELEASE'::transaction_type,
            'COMPLETED'::transaction_status,
            $2,
            $3,
            $4,
            'auction'::reference_type,
            $5,
            CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(user_id)
    .bind(amount_to_release)
    .bind(balance_after)
    .bind(auction_id)
    .bind(format!("Bid release for auction {}", auction_id))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn ensure_wallet_exists(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), BiddingError> {
    sqlx::query(
        r#"
        INSERT INTO wallets (user_id, balance, held_balance)
        VALUES ($1, 0, 0)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn convert_winner_hold_to_payment(
    tx: &mut Transaction<'_, Postgres>,
    winner_id: Uuid,
    auction_id: Uuid,
    amount: i64,
) -> Result<(), BiddingError> {
    if amount <= 0 {
        return Err(BiddingError::ValidationError(
            "winning amount must be greater than 0".to_string(),
        ));
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET balance = balance - $2,
            held_balance = held_balance - $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND held_balance >= $2
          AND balance >= $2
        RETURNING balance, held_balance
        "#,
    )
    .bind(winner_id)
    .bind(amount)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (
            wallet_id,
            type,
            status,
            amount,
            balance_after,
            reference_id,
            reference_type,
            description,
            completed_at
        )
        VALUES (
            $1,
            'BID_CONVERT'::transaction_type,
            'COMPLETED'::transaction_status,
            $2,
            $3,
            $4,
            'auction'::reference_type,
            $5,
            CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(winner_id)
    .bind(amount)
    .bind(balance_after)
    .bind(auction_id)
    .bind(format!("Bid convert for auction {}", auction_id))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn credit_seller_payment(
    tx: &mut Transaction<'_, Postgres>,
    seller_id: Uuid,
    auction_id: Uuid,
    amount: i64,
) -> Result<(), BiddingError> {
    if amount <= 0 {
        return Err(BiddingError::ValidationError(
            "payment amount must be greater than 0".to_string(),
        ));
    }

    ensure_wallet_exists(tx, seller_id).await?;

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET balance = balance + $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
        RETURNING balance
        "#,
    )
    .bind(seller_id)
    .bind(amount)
    .fetch_one(&mut **tx)
    .await?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    sqlx::query(
        r#"
        INSERT INTO wallet_transactions (
            wallet_id,
            type,
            status,
            amount,
            balance_after,
            reference_id,
            reference_type,
            description,
            completed_at
        )
        VALUES (
            $1,
            'PAYMENT_RECEIVED'::transaction_type,
            'COMPLETED'::transaction_status,
            $2,
            $3,
            $4,
            'auction'::reference_type,
            $5,
            CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(seller_id)
    .bind(amount)
    .bind(balance_after)
    .bind(auction_id)
    .bind(format!("Payment received from auction {}", auction_id))
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn finalize_auction_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    auction_id: Uuid,
    requested_by_user_id: Option<Uuid>,
    force: bool,
    now: DateTime<Utc>,
) -> Result<FinalizeAuctionResponse, BiddingError> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            listing_id,
            seller_id,
            seller_name,
            title,
            image_url,
            current_price,
            reserve_price,
            status::text AS status,
            ends_at,
            winner_id,
            winner_name
        FROM auctions
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(BiddingError::AuctionNotFound)?;

    let seller_id: Uuid = row.try_get("seller_id")?;
    if let Some(requester_id) = requested_by_user_id {
        if seller_id != requester_id {
            return Err(BiddingError::Unauthorized);
        }
    }

    let listing_id: Uuid = row.try_get("listing_id")?;
    let seller_name: String = row.try_get("seller_name")?;
    let title: String = row.try_get("title")?;
    let image_url: String = row.try_get("image_url")?;
    let reserve_price: Option<i64> = row.try_get("reserve_price")?;
    let status: String = row.try_get("status")?;
    let ends_at: DateTime<Utc> = row.try_get("ends_at")?;
    let existing_winner_id: Option<Uuid> = row.try_get("winner_id")?;
    let existing_winner_name: Option<String> = row.try_get("winner_name")?;
    let current_price: i64 = row.try_get("current_price")?;

    if status == "WON" || status == "UNSOLD" {
        let existing_order_id: Option<Uuid> = sqlx::query_scalar(
            r#"
            SELECT id
            FROM orders
            WHERE auction_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(auction_id)
        .fetch_optional(&mut **tx)
        .await?;

        return Ok(FinalizeAuctionResponse {
            auction_id,
            status,
            winner_id: existing_winner_id,
            winner_name: existing_winner_name,
            final_price: Some(current_price),
            order_id: existing_order_id,
        });
    }

    if status != "ACTIVE" && status != "EXTENDED" && status != "CLOSED" {
        return Err(BiddingError::AuctionNotActive);
    }

    if !force && now < ends_at {
        return Err(BiddingError::ValidationError(
            "auction cannot be finalized before endsAt".to_string(),
        ));
    }

    let top_bid = sqlx::query(
        r#"
        SELECT id, bidder_id, bidder_name, amount
        FROM bids
        WHERE auction_id = $1
        ORDER BY amount DESC, created_at ASC, id ASC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut **tx)
    .await?;

    let mut final_status = "UNSOLD".to_string();
    let mut winner_id: Option<Uuid> = None;
    let mut winner_name: Option<String> = None;
    let mut final_price: Option<i64> = None;
    let mut order_id: Option<Uuid> = None;

    if let Some(top_bid) = top_bid {
        let top_bidder_id: Uuid = top_bid.try_get("bidder_id")?;
        let top_bidder_name: String = top_bid.try_get("bidder_name")?;
        let top_amount: i64 = top_bid.try_get("amount")?;
        final_price = Some(top_amount);

        let reserve_met = reserve_price
            .map(|reserve| top_amount >= reserve)
            .unwrap_or(true);
        if reserve_met {
            final_status = "WON".to_string();
            winner_id = Some(top_bidder_id);
            winner_name = Some(top_bidder_name.clone());

            convert_winner_hold_to_payment(tx, top_bidder_id, auction_id, top_amount).await?;
            credit_seller_payment(tx, seller_id, auction_id, top_amount).await?;

            let existing_order: Option<Uuid> = sqlx::query_scalar(
                r#"
                SELECT id
                FROM orders
                WHERE auction_id = $1
                ORDER BY created_at DESC
                LIMIT 1
                "#,
            )
            .bind(auction_id)
            .fetch_optional(&mut **tx)
            .await?;

            if let Some(existing_order) = existing_order {
                order_id = Some(existing_order);
            } else {
                let inserted_order = sqlx::query(
                    r#"
                    INSERT INTO orders (
                        auction_id,
                        listing_id,
                        buyer_id,
                        buyer_name,
                        seller_id,
                        seller_name,
                        title,
                        image_url,
                        final_price,
                        status,
                        paid_at
                    )
                    VALUES (
                        $1, $2, $3, $4, $5, $6, $7, $8, $9, 'PAID'::order_status, CURRENT_TIMESTAMP
                    )
                    RETURNING id
                    "#,
                )
                .bind(auction_id)
                .bind(listing_id)
                .bind(top_bidder_id)
                .bind(top_bidder_name)
                .bind(seller_id)
                .bind(seller_name)
                .bind(&title)
                .bind(image_url)
                .bind(top_amount)
                .fetch_one(&mut **tx)
                .await?;

                order_id = Some(inserted_order.try_get("id")?);
            }
        } else {
            release_wallet_for_outbid(tx, top_bidder_id, auction_id, top_amount).await?;
        }
    }

    sqlx::query(
        r#"
        UPDATE auctions
        SET
            status = $2::auction_status,
            winner_id = $3,
            winner_name = $4,
            ends_at = CASE WHEN $5 THEN CURRENT_TIMESTAMP ELSE ends_at END
        WHERE id = $1
        "#,
    )
    .bind(auction_id)
    .bind(final_status.as_str())
    .bind(winner_id)
    .bind(winner_name.clone())
    .bind(force)
    .execute(&mut **tx)
    .await?;

    let listing_status = if final_status == "WON" {
        "SOLD"
    } else {
        "EXPIRED"
    };
    let listing_price = final_price.unwrap_or(current_price);

    sqlx::query(
        r#"
        UPDATE listings
        SET
            status = $2::listing_status,
            current_price = $3,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $1
        "#,
    )
    .bind(listing_id)
    .bind(listing_status)
    .bind(listing_price)
    .execute(&mut **tx)
    .await?;

    if final_status == "WON" {
        if let Some(winner_id) = winner_id {
            create_notification(
                tx,
                winner_id,
                "AUCTION_WON",
                "You won the auction",
                &format!(
                    "You have won '{}' with final price {}.",
                    title,
                    final_price.unwrap_or(current_price)
                ),
                auction_id,
            )
            .await?;

            let losing_bidders: Vec<Uuid> = sqlx::query_scalar(
                r#"
                SELECT DISTINCT bidder_id
                FROM bids
                WHERE auction_id = $1
                  AND bidder_id <> $2
                "#,
            )
            .bind(auction_id)
            .bind(winner_id)
            .fetch_all(&mut **tx)
            .await?;

            for bidder_id in losing_bidders {
                create_notification(
                    tx,
                    bidder_id,
                    "AUCTION_LOST",
                    "Auction ended",
                    &format!("Auction '{}' has closed and another bidder won.", title),
                    auction_id,
                )
                .await?;
            }
        }

        create_notification(
            tx,
            seller_id,
            "PAYMENT_RECEIVED",
            "Auction payment received",
            &format!(
                "Your listing '{}' has sold and payment {} has been recorded.",
                title,
                final_price.unwrap_or(current_price)
            ),
            auction_id,
        )
        .await?;
    } else {
        let participants: Vec<Uuid> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT bidder_id
            FROM bids
            WHERE auction_id = $1
            "#,
        )
        .bind(auction_id)
        .fetch_all(&mut **tx)
        .await?;

        for bidder_id in participants {
            create_notification(
                tx,
                bidder_id,
                "AUCTION_LOST",
                "Auction ended without winner",
                &format!(
                    "Auction '{}' closed without winner because reserve was not met.",
                    title
                ),
                auction_id,
            )
            .await?;
        }
    }

    Ok(FinalizeAuctionResponse {
        auction_id,
        status: final_status,
        winner_id,
        winner_name,
        final_price,
        order_id,
    })
}

pub(crate) async fn finalize_auction_as_system(
    pool: &sqlx::postgres::PgPool,
    auction_id: Uuid,
    amqp: Option<&crate::infrastructure::amqp::AmqpPublisher>,
) -> Result<FinalizeAuctionResponse, BiddingError> {
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    let response = finalize_auction_in_tx(&mut tx, auction_id, None, false, now).await?;
    tx.commit().await?;

    if let Some(publisher) = amqp {
        publisher.publish(
            "auction.finalized",
            serde_json::json!({
                "auctionId": response.auction_id.to_string(),
                "status": response.status,
                "winnerId": response.winner_id.map(|id| id.to_string()),
                "winnerName": response.winner_name,
                "finalPrice": response.final_price,
            }),
        );
    }

    Ok(response)
}

async fn fetch_auction_snapshot(
    state: &AppState,
    auction_id: Uuid,
    user_id: Uuid,
) -> Result<AuctionDetailResponse, BiddingError> {
    let row = sqlx::query(
        r#"
        SELECT
            id,
            listing_id,
            seller_id,
            seller_name,
            title,
            description,
            image_url,
            start_price,
            current_price,
            reserve_price,
            bid_increment,
            bid_count,
            status::text AS status,
            winner_id,
            winner_name,
            starts_at,
            ends_at,
            original_ends_at,
            extension_count,
            created_at
        FROM auctions
        WHERE id = $1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(BiddingError::AuctionNotFound)?;

    let highest_bidder_alias: Option<String> = sqlx::query_scalar(
        r#"
        SELECT bidder_name
        FROM bids
        WHERE auction_id = $1
        ORDER BY amount DESC, created_at ASC, id ASC
        LIMIT 1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&state.pool)
    .await?;

    let my_latest_bid: Option<i64> = sqlx::query_scalar(
        r#"
        SELECT amount
        FROM bids
        WHERE auction_id = $1 AND bidder_id = $2
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(auction_id)
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(AuctionDetailResponse {
        id: row.try_get("id")?,
        listing_id: row.try_get("listing_id")?,
        seller_id: row.try_get("seller_id")?,
        seller_name: row.try_get("seller_name")?,
        title: row.try_get("title")?,
        description: row.try_get("description")?,
        image_url: row.try_get("image_url")?,
        start_price: row.try_get("start_price")?,
        current_price: row.try_get("current_price")?,
        reserve_price: row.try_get("reserve_price")?,
        bid_increment: row.try_get("bid_increment")?,
        bid_count: row.try_get("bid_count")?,
        status: row.try_get("status")?,
        winner_id: row.try_get("winner_id")?,
        winner_name: row.try_get("winner_name")?,
        starts_at: row.try_get("starts_at")?,
        ends_at: row.try_get("ends_at")?,
        original_ends_at: row.try_get("original_ends_at")?,
        extension_count: row.try_get("extension_count")?,
        created_at: row.try_get("created_at")?,
        highest_bidder_alias,
        my_latest_bid,
    })
}

pub async fn get_auction_detail(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<AuctionDetailResponse>, BiddingError> {
    let auction = fetch_auction_snapshot(&state, auction_id, user.id).await?;
    Ok(Json(auction))
}

pub async fn get_auction_history(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<AuctionHistoryResponse>, BiddingError> {
    let auction = fetch_auction_snapshot(&state, auction_id, user.id).await?;

    let rows = sqlx::query(
        r#"
        SELECT
            id,
            auction_id,
            bidder_id,
            bidder_name,
            amount,
            is_proxy,
            is_winning,
            created_at,
            ROW_NUMBER() OVER (ORDER BY created_at ASC, id ASC) AS received_sequence
        FROM bids
        WHERE auction_id = $1
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(auction_id)
    .fetch_all(&state.pool)
    .await?;

    let bids = rows
        .into_iter()
        .map(|row| -> Result<BidHistoryEntryResponse, sqlx::Error> {
            let is_winning: bool = row.try_get("is_winning")?;
            let bidder_id: Uuid = row.try_get("bidder_id")?;
            Ok(BidHistoryEntryResponse {
                id: row.try_get("id")?,
                auction_id: row.try_get("auction_id")?,
                bidder_id,
                bidder_name: row.try_get("bidder_name")?,
                amount: row.try_get("amount")?,
                is_proxy: row.try_get("is_proxy")?,
                is_winning,
                status: if is_winning {
                    "LEADING".to_string()
                } else {
                    "OUTBID".to_string()
                },
                accepted_at: row.try_get("created_at")?,
                received_sequence: row.try_get("received_sequence")?,
                is_my_bid: bidder_id == user.id,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(AuctionHistoryResponse {
        auction,
        bids,
        ordering: AuctionHistoryOrderingResponse {
            primary: "acceptedAt DESC".to_string(),
            secondary: "receivedSequence DESC".to_string(),
        },
    }))
}

pub async fn place_bid(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<PlaceBidRequest>,
) -> Result<Json<PlaceBidResponse>, BiddingError> {
    if payload.amount <= 0 {
        return Err(BiddingError::ValidationError(
            "amount must be greater than 0".to_string(),
        ));
    }

    if let Some(max_amount) = payload.max_amount {
        if max_amount <= 0 {
            return Err(BiddingError::ValidationError(
                "maxAmount must be greater than 0".to_string(),
            ));
        }
        if max_amount < payload.amount {
            return Err(BiddingError::ValidationError(
                "maxAmount must be greater than or equal to amount".to_string(),
            ));
        }
    }

    let mut tx = state.pool.begin().await?;
    let now = Utc::now();

    let row = sqlx::query(
        r#"
        SELECT
            current_price,
            bid_increment,
            ends_at,
            status::text AS status,
            bid_count,
            extension_count
        FROM auctions
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(BiddingError::AuctionNotFound)?;

    let current_price: i64 = row.try_get("current_price")?;
    let bid_increment: i64 = row.try_get("bid_increment")?;
    let ends_at: DateTime<Utc> = row.try_get("ends_at")?;
    let status: String = row.try_get("status")?;
    let bid_count: i32 = row.try_get("bid_count")?;
    let extension_count: i32 = row.try_get("extension_count")?;

    if status != "ACTIVE" && status != "EXTENDED" {
        return Err(BiddingError::AuctionNotActive);
    }

    if now >= ends_at {
        return Err(BiddingError::AuctionEnded);
    }

    let minimum_allowed = current_price.saturating_add(bid_increment);
    if payload.amount < minimum_allowed {
        return Err(BiddingError::BidTooLow);
    }

    if let Some(max_amount) = payload.max_amount {
        sqlx::query(
            r#"
            INSERT INTO proxy_bids (auction_id, bidder_id, max_amount, is_active)
            VALUES ($1, $2, $3, true)
            ON CONFLICT (auction_id, bidder_id, is_active)
            DO UPDATE
                SET max_amount = EXCLUDED.max_amount,
                    updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(auction_id)
        .bind(user.id)
        .bind(max_amount)
        .execute(&mut *tx)
        .await?;
    }

    let previous_winning = sqlx::query(
        r#"
        SELECT bidder_id, amount
        FROM bids
        WHERE auction_id = $1 AND is_winning = true
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut *tx)
    .await?;

    let previous_winner_id: Option<Uuid> = previous_winning
        .as_ref()
        .and_then(|row| row.try_get("bidder_id").ok());
    let previous_winning_amount: i64 = previous_winning
        .as_ref()
        .and_then(|row| row.try_get("amount").ok())
        .unwrap_or(current_price);

    if let Some(previous_winner_id) = previous_winner_id {
        if previous_winner_id == user.id {
            let additional_hold = payload.amount.saturating_sub(previous_winning_amount);
            hold_wallet_for_bid(&mut tx, user.id, auction_id, additional_hold).await?;
        } else {
            release_wallet_for_outbid(
                &mut tx,
                previous_winner_id,
                auction_id,
                previous_winning_amount,
            )
            .await?;
            create_notification(
                &mut tx,
                previous_winner_id,
                "BID_OUTBID",
                "You were outbid",
                &format!(
                    "A higher bid has replaced your leading position in auction {}.",
                    auction_id
                ),
                auction_id,
            )
            .await?;
            hold_wallet_for_bid(&mut tx, user.id, auction_id, payload.amount).await?;
        }
    } else {
        hold_wallet_for_bid(&mut tx, user.id, auction_id, payload.amount).await?;
    }

    let inserted = sqlx::query(
        r#"
        INSERT INTO bids (auction_id, bidder_id, bidder_name, amount, is_proxy, is_winning)
        VALUES ($1, $2, $3, $4, false, true)
        RETURNING id
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .bind(user.name)
    .bind(payload.amount)
    .fetch_one(&mut *tx)
    .await?;

    let bid_id: Uuid = inserted.try_get("id")?;

    sqlx::query(
        r#"
        UPDATE bids
        SET is_winning = false
        WHERE auction_id = $1 AND id <> $2 AND is_winning = true
        "#,
    )
    .bind(auction_id)
    .bind(bid_id)
    .execute(&mut *tx)
    .await?;

    // Anti-sniping (spec §3 / WBS 3.1.4): "perpanjangan 2 menit dari waktu
    // penawaran tersebut diterima". When a bid lands in the last 2-minute
    // window, the auction is extended so it has 2 full minutes from `now`
    // remaining. `max(ends_at, ...)` keeps the existing end if it is somehow
    // later than `now + 2 min`.
    let remaining = ends_at - now;
    let should_extend = remaining <= Duration::minutes(2);
    let next_ends_at = if should_extend {
        std::cmp::max(ends_at, now + Duration::minutes(2))
    } else {
        ends_at
    };
    let next_extension_count = if should_extend {
        extension_count + 1
    } else {
        extension_count
    };
    let next_status = if should_extend {
        "EXTENDED"
    } else {
        status.as_str()
    };
    let next_bid_count = bid_count + 1;

    sqlx::query(
        r#"
        UPDATE auctions
        SET
            current_price = $2,
            bid_count = $3,
            ends_at = $4,
            extension_count = $5,
            status = $6::auction_status
        WHERE id = $1
        "#,
    )
    .bind(auction_id)
    .bind(payload.amount)
    .bind(next_bid_count)
    .bind(next_ends_at)
    .bind(next_extension_count)
    .bind(next_status)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        UPDATE listings
        SET
            current_price = $2,
            bid_count = $3,
            ends_at = $4,
            updated_at = NOW()
        WHERE auction_id = $1
        "#,
    )
    .bind(auction_id)
    .bind(payload.amount)
    .bind(next_bid_count)
    .bind(next_ends_at)
    .execute(&mut *tx)
    .await?;

    let mut effective_current_price = payload.amount;
    let mut effective_bid_count = next_bid_count;
    let mut effective_winner_id = user.id;
    let mut auto_bid_applied = false;

    for _ in 0..16 {
        let current_winning = sqlx::query(
            r#"
            SELECT bidder_id, bidder_name, amount, created_at
            FROM bids
            WHERE auction_id = $1 AND is_winning = true
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            FOR UPDATE
            "#,
        )
        .bind(auction_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(BiddingError::BidNotFound)?;

        let current_winner_id: Uuid = current_winning.try_get("bidder_id")?;
        let current_winner_name: String = current_winning.try_get("bidder_name")?;
        let current_winning_amount: i64 = current_winning.try_get("amount")?;
        let current_winning_created_at: DateTime<Utc> = current_winning.try_get("created_at")?;

        effective_current_price = current_winning_amount;
        effective_winner_id = current_winner_id;

        let mut capabilities = fetch_active_proxy_capabilities(&mut tx, auction_id).await?;
        if let Some(capability) = capabilities
            .iter_mut()
            .find(|capability| capability.bidder_id == current_winner_id)
        {
            if capability.max_amount < current_winning_amount {
                capability.max_amount = current_winning_amount;
            }
            if capability.bidder_name.is_empty() {
                capability.bidder_name = current_winner_name.clone();
            }
        } else {
            capabilities.push(ProxyCapability {
                bidder_id: current_winner_id,
                bidder_name: current_winner_name.clone(),
                max_amount: current_winning_amount,
                priority_at: current_winning_created_at,
                has_active_proxy: false,
            });
        }

        sort_proxy_capabilities(&mut capabilities);
        let Some(top_bidder) = capabilities.first().cloned() else {
            break;
        };

        let second_max = capabilities
            .get(1)
            .map(|capability| capability.max_amount)
            .unwrap_or(0);
        let competition_floor = second_max.saturating_add(bid_increment);
        let target_price = current_winning_amount.max(top_bidder.max_amount.min(competition_floor));

        if target_price <= current_winning_amount {
            break;
        }

        if top_bidder.bidder_id == current_winner_id {
            let additional_hold = target_price.saturating_sub(current_winning_amount);
            match hold_wallet_for_bid(&mut tx, current_winner_id, auction_id, additional_hold).await
            {
                Ok(()) => {}
                Err(BiddingError::InsufficientBalance) if top_bidder.has_active_proxy => {
                    deactivate_proxy_bid(&mut tx, auction_id, top_bidder.bidder_id).await?;
                    continue;
                }
                Err(err) => return Err(err),
            }
        } else {
            match hold_wallet_for_bid(&mut tx, top_bidder.bidder_id, auction_id, target_price).await
            {
                Ok(()) => {}
                Err(BiddingError::InsufficientBalance) if top_bidder.has_active_proxy => {
                    deactivate_proxy_bid(&mut tx, auction_id, top_bidder.bidder_id).await?;
                    continue;
                }
                Err(err) => return Err(err),
            }

            release_wallet_for_outbid(
                &mut tx,
                current_winner_id,
                auction_id,
                current_winning_amount,
            )
            .await?;
            create_notification(
                &mut tx,
                current_winner_id,
                "BID_OUTBID",
                "You were outbid",
                &format!(
                    "An automatic proxy bid has replaced your lead in auction {}.",
                    auction_id
                ),
                auction_id,
            )
            .await?;
        }

        let auto_bidder_name = if top_bidder.bidder_name.is_empty() {
            "Proxy bidder".to_string()
        } else {
            top_bidder.bidder_name
        };
        let auto_bid = sqlx::query(
            r#"
            INSERT INTO bids (auction_id, bidder_id, bidder_name, amount, is_proxy, is_winning)
            VALUES ($1, $2, $3, $4, true, true)
            RETURNING id
            "#,
        )
        .bind(auction_id)
        .bind(top_bidder.bidder_id)
        .bind(auto_bidder_name)
        .bind(target_price)
        .fetch_one(&mut *tx)
        .await?;
        let auto_bid_id: Uuid = auto_bid.try_get("id")?;

        sqlx::query(
            r#"
            UPDATE bids
            SET is_winning = false
            WHERE auction_id = $1
              AND id <> $2
              AND is_winning = true
            "#,
        )
        .bind(auction_id)
        .bind(auto_bid_id)
        .execute(&mut *tx)
        .await?;

        effective_current_price = target_price;
        effective_winner_id = top_bidder.bidder_id;
        effective_bid_count = effective_bid_count.saturating_add(1);
        auto_bid_applied = true;

        sqlx::query(
            r#"
            UPDATE auctions
            SET
                current_price = $2,
                bid_count = $3
            WHERE id = $1
            "#,
        )
        .bind(auction_id)
        .bind(effective_current_price)
        .bind(effective_bid_count)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE listings
            SET
                current_price = $2,
                bid_count = $3,
                updated_at = NOW()
            WHERE auction_id = $1
            "#,
        )
        .bind(auction_id)
        .bind(effective_current_price)
        .bind(effective_bid_count)
        .execute(&mut *tx)
        .await?;

        break;
    }

    tx.commit().await?;

    if let Some(ref amqp) = state.amqp {
        amqp.publish(
            "auction.bid_placed",
            serde_json::json!({
                "auctionId": auction_id.to_string(),
                "currentPrice": effective_current_price,
                "bidCount": effective_bid_count,
                "minimumNextBid": effective_current_price.saturating_add(bid_increment),
                "extended": should_extend,
                "endsAt": next_ends_at.to_rfc3339(),
                "extensionCount": next_extension_count,
            }),
        );
    }

    Ok(Json(PlaceBidResponse {
        bid_id,
        auction_id,
        current_price: effective_current_price,
        bid_count: effective_bid_count,
        ends_at: next_ends_at,
        extended: should_extend,
        extension_count: next_extension_count,
        minimum_next_bid: effective_current_price.saturating_add(bid_increment),
        auto_bid_applied,
        effective_winner_id,
    }))
}

pub async fn get_my_proxy_bid(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<ProxyBidResponse>, BiddingError> {
    let row = sqlx::query(
        r#"
        SELECT
            a.status::text AS auction_status,
            a.current_price,
            a.bid_increment,
            pb.max_amount,
            pb.updated_at,
            (
                SELECT b.bidder_id
                FROM bids b
                WHERE b.auction_id = a.id AND b.is_winning = true
                ORDER BY b.created_at DESC, b.id DESC
                LIMIT 1
            ) AS leading_bidder_id
        FROM auctions a
        LEFT JOIN proxy_bids pb
            ON pb.auction_id = a.id
           AND pb.bidder_id = $2
           AND pb.is_active = true
        WHERE a.id = $1
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(BiddingError::AuctionNotFound)?;

    let current_price: i64 = row.try_get("current_price")?;
    let bid_increment: i64 = row.try_get("bid_increment")?;
    let max_amount: Option<i64> = row.try_get("max_amount")?;
    let leading_bidder_id: Option<Uuid> = row.try_get("leading_bidder_id")?;

    Ok(Json(ProxyBidResponse {
        auction_id,
        bidder_id: user.id,
        enabled: max_amount.is_some(),
        max_amount,
        auction_status: row.try_get("auction_status")?,
        current_price,
        minimum_proxy_amount: current_price.saturating_add(bid_increment),
        currently_leading: leading_bidder_id == Some(user.id),
        updated_at: row.try_get("updated_at")?,
    }))
}

pub async fn upsert_my_proxy_bid(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
    Json(payload): Json<UpsertProxyBidRequest>,
) -> Result<Json<ProxyBidResponse>, BiddingError> {
    if payload.max_amount <= 0 {
        return Err(BiddingError::ValidationError(
            "maxAmount must be greater than 0".to_string(),
        ));
    }

    let now = Utc::now();
    let mut tx = state.pool.begin().await?;

    let auction_row = sqlx::query(
        r#"
        SELECT
            status::text AS status,
            current_price,
            bid_increment,
            ends_at
        FROM auctions
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(BiddingError::AuctionNotFound)?;

    let auction_status: String = auction_row.try_get("status")?;
    let current_price: i64 = auction_row.try_get("current_price")?;
    let bid_increment: i64 = auction_row.try_get("bid_increment")?;
    let ends_at: DateTime<Utc> = auction_row.try_get("ends_at")?;
    if auction_status != "ACTIVE" && auction_status != "EXTENDED" {
        return Err(BiddingError::AuctionNotActive);
    }
    if now >= ends_at {
        return Err(BiddingError::AuctionEnded);
    }

    let leading_row = sqlx::query(
        r#"
        SELECT bidder_id, amount
        FROM bids
        WHERE auction_id = $1 AND is_winning = true
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&mut *tx)
    .await?;

    let leading_bidder_id: Option<Uuid> = leading_row
        .as_ref()
        .and_then(|row| row.try_get("bidder_id").ok());
    let leading_amount: Option<i64> = leading_row
        .as_ref()
        .and_then(|row| row.try_get("amount").ok());
    let currently_leading = leading_bidder_id == Some(user.id);

    let minimum_proxy_amount = compute_minimum_proxy_amount(
        current_price,
        bid_increment,
        currently_leading,
        leading_amount,
    );
    if payload.max_amount < minimum_proxy_amount {
        return Err(BiddingError::ValidationError(format!(
            "maxAmount must be at least {minimum_proxy_amount}"
        )));
    }

    let row = sqlx::query(
        r#"
        INSERT INTO proxy_bids (auction_id, bidder_id, max_amount, is_active)
        VALUES ($1, $2, $3, true)
        ON CONFLICT (auction_id, bidder_id, is_active)
        DO UPDATE
            SET max_amount = EXCLUDED.max_amount,
                updated_at = CURRENT_TIMESTAMP
        RETURNING max_amount, updated_at
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .bind(payload.max_amount)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(ProxyBidResponse {
        auction_id,
        bidder_id: user.id,
        enabled: true,
        max_amount: Some(row.try_get("max_amount")?),
        auction_status,
        current_price,
        minimum_proxy_amount: current_price.saturating_add(bid_increment),
        currently_leading,
        updated_at: row.try_get("updated_at")?,
    }))
}

pub async fn disable_my_proxy_bid(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<DisableProxyBidResponse>, BiddingError> {
    let auction_exists: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id
        FROM auctions
        WHERE id = $1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&state.pool)
    .await?;

    if auction_exists.is_none() {
        return Err(BiddingError::AuctionNotFound);
    }

    let result = sqlx::query(
        r#"
        UPDATE proxy_bids
        SET is_active = false,
            updated_at = CURRENT_TIMESTAMP
        WHERE auction_id = $1
          AND bidder_id = $2
          AND is_active = true
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(DisableProxyBidResponse {
        auction_id,
        bidder_id: user.id,
        disabled: result.rows_affected() > 0,
    }))
}

pub async fn finalize_auction(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<FinalizeAuctionQuery>,
) -> Result<Json<FinalizeAuctionResponse>, BiddingError> {
    let force = query.force.unwrap_or(false);
    let now = Utc::now();
    let mut tx = state.pool.begin().await?;
    let response = finalize_auction_in_tx(&mut tx, auction_id, Some(user.id), force, now).await?;
    tx.commit().await?;

    if let Some(ref amqp) = state.amqp {
        amqp.publish(
            "auction.finalized",
            serde_json::json!({
                "auctionId": response.auction_id.to_string(),
                "status": response.status,
                "winnerId": response.winner_id.map(|id| id.to_string()),
                "winnerName": response.winner_name,
                "finalPrice": response.final_price,
            }),
        );
    }

    Ok(Json(response))
}

pub async fn list_my_bids(
    State(state): State<AppState>,
    Extension(user): Extension<AuthUser>,
    Query(query): Query<MyBidQuery>,
) -> Result<Json<MyBidListResponse>, BiddingError> {
    let rows = sqlx::query(
        r#"
        SELECT
            a.id AS auction_id,
            a.listing_id,
            a.seller_id,
            a.seller_name,
            a.title,
            a.description,
            a.image_url,
            a.start_price,
            a.current_price,
            a.reserve_price,
            a.bid_increment,
            a.bid_count,
            a.status::text AS auction_status,
            a.ends_at,
            a.starts_at,
            a.original_ends_at,
            a.extension_count,
            a.winner_id,
            a.winner_name,
            (
                SELECT b.amount
                FROM bids b
                WHERE b.auction_id = a.id AND b.bidder_id = $1
                ORDER BY b.created_at DESC, b.id DESC
                LIMIT 1
            ) AS my_latest_bid,
            (
                SELECT b.bidder_id
                FROM bids b
                WHERE b.auction_id = a.id
                ORDER BY b.amount DESC, b.created_at ASC, b.id ASC
                LIMIT 1
            ) AS leading_bidder_id,
            (
                SELECT b.bidder_name
                FROM bids b
                WHERE b.auction_id = a.id
                ORDER BY b.amount DESC, b.created_at ASC, b.id ASC
                LIMIT 1
            ) AS highest_bidder_alias,
            (
                SELECT b.created_at
                FROM bids b
                WHERE b.auction_id = a.id AND b.bidder_id = $1
                ORDER BY b.created_at DESC, b.id DESC
                LIMIT 1
            ) AS last_bid_at
        FROM auctions a
        WHERE EXISTS (
            SELECT 1
            FROM bids ub
            WHERE ub.auction_id = a.id AND ub.bidder_id = $1
        )
        ORDER BY a.ends_at DESC
        "#,
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let all_items = rows
        .into_iter()
        .map(|row| -> Result<MyBidItemResponse, sqlx::Error> {
            let auction_status: String = row.try_get("auction_status")?;
            let leading_bidder_id: Option<Uuid> = row.try_get("leading_bidder_id")?;
            let winner_id: Option<Uuid> = row.try_get("winner_id")?;
            let my_bid_status =
                classify_my_bid_status(&auction_status, leading_bidder_id, winner_id, user.id);
            let reserve_price: Option<i64> = row.try_get("reserve_price")?;
            let current_price: i64 = row.try_get("current_price")?;
            let starts_at: DateTime<Utc> = row.try_get("starts_at")?;
            let last_bid_at: Option<DateTime<Utc>> = row.try_get("last_bid_at")?;

            Ok(MyBidItemResponse {
                auction_id: row.try_get("auction_id")?,
                listing_id: row.try_get("listing_id")?,
                seller_id: row.try_get("seller_id")?,
                seller_name: row.try_get("seller_name")?,
                title: row.try_get("title")?,
                description: row.try_get("description")?,
                image_url: row.try_get("image_url")?,
                start_price: row.try_get("start_price")?,
                current_price,
                reserve_price,
                bid_increment: row.try_get("bid_increment")?,
                bid_count: row.try_get("bid_count")?,
                auction_status,
                my_bid_status,
                winner_id,
                ends_at: row.try_get("ends_at")?,
                starts_at,
                original_ends_at: row.try_get("original_ends_at")?,
                extension_count: row.try_get("extension_count")?,
                highest_bidder_alias: row.try_get("highest_bidder_alias")?,
                winner_name: row.try_get("winner_name")?,
                my_latest_bid: row.try_get("my_latest_bid")?,
                last_bid_at: last_bid_at.unwrap_or(starts_at),
                is_reserve_met: reserve_price
                    .map(|reserve| current_price >= reserve)
                    .unwrap_or(true),
                currency: "IDR".to_string(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let summary = MyBidSummaryResponse {
        all: all_items.len() as i64,
        winning: all_items
            .iter()
            .filter(|item| item.my_bid_status == "WINNING")
            .count() as i64,
        outbid: all_items
            .iter()
            .filter(|item| item.my_bid_status == "OUTBID")
            .count() as i64,
        won: all_items
            .iter()
            .filter(|item| item.my_bid_status == "WON")
            .count() as i64,
        lost: all_items
            .iter()
            .filter(|item| item.my_bid_status == "LOST")
            .count() as i64,
    };

    let filter = query.status.as_deref().map(str::to_ascii_lowercase);
    let data = match filter.as_deref() {
        Some("winning") => all_items
            .into_iter()
            .filter(|item| item.my_bid_status == "WINNING")
            .collect(),
        Some("outbid") => all_items
            .into_iter()
            .filter(|item| item.my_bid_status == "OUTBID")
            .collect(),
        Some("won") => all_items
            .into_iter()
            .filter(|item| item.my_bid_status == "WON")
            .collect(),
        Some("lost") => all_items
            .into_iter()
            .filter(|item| item.my_bid_status == "LOST")
            .collect(),
        Some(other) => {
            return Err(BiddingError::ValidationError(format!(
                "status filter '{other}' is not supported"
            )));
        }
        None => all_items,
    };

    Ok(Json(MyBidListResponse {
        user: MyBidUserResponse {
            id: user.id,
            name: user.name,
        },
        data,
        summary,
    }))
}

pub async fn get_my_bid_detail(
    State(state): State<AppState>,
    Path(auction_id): Path<Uuid>,
    Extension(user): Extension<AuthUser>,
) -> Result<Json<MyBidDetailResponse>, BiddingError> {
    let auction = fetch_auction_snapshot(&state, auction_id, user.id).await?;

    let my_latest_bid: i64 = sqlx::query_scalar(
        r#"
        SELECT amount
        FROM bids
        WHERE auction_id = $1 AND bidder_id = $2
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(BiddingError::BidNotFound)?;

    let leading_bidder_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT bidder_id
        FROM bids
        WHERE auction_id = $1
        ORDER BY amount DESC, created_at ASC, id ASC
        LIMIT 1
        "#,
    )
    .bind(auction_id)
    .fetch_optional(&state.pool)
    .await?;

    let my_bid_status = classify_my_bid_status(
        &auction.status,
        leading_bidder_id,
        auction.winner_id,
        user.id,
    );

    let placed_bid_count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM bids
        WHERE auction_id = $1 AND bidder_id = $2
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;

    let timeline_rows = sqlx::query(
        r#"
        SELECT id, amount, is_winning, created_at
        FROM bids
        WHERE auction_id = $1 AND bidder_id = $2
        ORDER BY created_at DESC, id DESC
        "#,
    )
    .bind(auction_id)
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let mut timeline = timeline_rows
        .into_iter()
        .map(|row| -> Result<MyBidTimelineEventResponse, sqlx::Error> {
            let is_winning: bool = row.try_get("is_winning")?;
            let amount: i64 = row.try_get("amount")?;
            let event_type = if is_winning { "LEADING" } else { "PLACED_BID" };
            let note = if is_winning {
                "Bid is currently leading".to_string()
            } else {
                "Bid submitted".to_string()
            };

            Ok(MyBidTimelineEventResponse {
                at: row.try_get("created_at")?,
                r#type: event_type.to_string(),
                amount: Some(amount),
                note,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    if my_bid_status == "OUTBID" {
        timeline.insert(
            0,
            MyBidTimelineEventResponse {
                at: Utc::now(),
                r#type: "OUTBID".to_string(),
                amount: None,
                note: "Another bidder is currently leading.".to_string(),
            },
        );
    }

    let winning_gap = if auction.current_price > my_latest_bid {
        auction.current_price - my_latest_bid
    } else {
        0
    };

    Ok(Json(MyBidDetailResponse {
        is_reserve_met: auction
            .reserve_price
            .map(|reserve| auction.current_price >= reserve)
            .unwrap_or(true),
        winning_gap,
        my_bid_status,
        my_latest_bid,
        placed_bid_count,
        timeline,
        auction,
    }))
}

#[cfg(test)]
mod tests;
