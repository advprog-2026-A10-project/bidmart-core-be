use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::{Listing, ListingStatus};
use crate::modules::catalog::application::dto::listing_dto::ListingImageResponse;

#[derive(Debug, Serialize)]
pub struct BuyerListingResponse {
    pub id: Uuid,
    pub seller_name: String,
    pub category_id: Option<i32>,
    pub category_name: String,
    pub title: String,
    pub description: String,
    pub start_price: i64,
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

impl From<Listing> for BuyerListingResponse {
    fn from(l: Listing) -> Self {
        Self {
            id: l.id,
            seller_name: l.seller_name,
            category_id: l.category_id,
            category_name: l.category_name,
            title: l.title,
            description: l.description,
            start_price: l.start_price,
            current_price: l.current_price,
            min_increment: l.min_increment,
            bid_count: l.bid_count,
            status: l.status,
            auction_id: l.auction_id,
            starts_at: l.starts_at,
            ends_at: l.ends_at,
            created_at: l.created_at,
            updated_at: l.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BuyerListingDetailResponse {
    #[serde(flatten)]
    pub listing: BuyerListingResponse,
    pub images: Vec<ListingImageResponse>,
}

#[derive(Debug, Deserialize, Default)]
pub struct PublicListingQueryParams {
    pub q: Option<String>,
    pub category_id: Option<i32>,
    #[serde(rename = "min")]
    pub min_price: Option<i64>,
    #[serde(rename = "max")]
    pub max_price: Option<i64>,
    #[serde(rename = "endBefore")]
    pub end_before: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CatalogPageParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
