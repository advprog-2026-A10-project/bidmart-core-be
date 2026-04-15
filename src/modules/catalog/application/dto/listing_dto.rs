use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::{Listing, ListingImage, ListingStatus};

// Request DTOs

#[derive(Debug, Deserialize)]
pub struct CreateListingRequest {
    pub category_id: Option<i32>,
    pub title: String,
    pub description: String,
    pub image_urls: Vec<String>,
    pub start_price: i64,
    pub reserve_price: Option<i64>,
    pub min_increment: i64,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateListingRequest {
    pub description: Option<String>,
    pub image_urls: Option<Vec<String>>,
}

// Query / Pagination

/// Query params untuk list_my_listings (seller).
#[derive(Debug, Deserialize, Default)]
pub struct ListingQueryParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

// Response DTOs

#[derive(Debug, Serialize)]
pub struct ListingImageResponse {
    pub id: Uuid,
    pub url: String,
    pub order: i32,
}

impl From<ListingImage> for ListingImageResponse {
    fn from(img: ListingImage) -> Self {
        Self {
            id: img.id,
            url: img.url,
            order: img.order,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ListingResponse {
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

impl From<Listing> for ListingResponse {
    fn from(l: Listing) -> Self {
        Self {
            id: l.id,
            seller_id: l.seller_id,
            seller_name: l.seller_name,
            category_id: l.category_id,
            category_name: l.category_name,
            title: l.title,
            description: l.description,
            start_price: l.start_price,
            reserve_price: l.reserve_price,
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
pub struct ListingDetailResponse {
    #[serde(flatten)]
    pub listing: ListingResponse,
    pub images: Vec<ListingImageResponse>,
}