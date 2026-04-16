use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "listing_status", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ListingStatus {
    Draft,
    Active,
    Sold,
    Cancelled,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Listing {
    pub id: Uuid,
    pub seller_id: Uuid,
    pub seller_name: String,
    pub category_id: Option<i32>,
    pub category_name: String,
    pub title: String,
    pub description: String,
    pub start_price: i64,
    pub reserve_price: Option<i64>,
    pub current_price: i64,
    pub min_increment: i64,
    pub bid_count: i32,
    pub status: ListingStatus,
    pub auction_id: Option<Uuid>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
