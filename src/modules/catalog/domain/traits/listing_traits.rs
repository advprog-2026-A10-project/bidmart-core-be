use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::{Listing, ListingStatus};
use crate::modules::catalog::domain::errors::ListingError;

pub struct ListingFilter {
    pub keyword: Option<String>,
    pub category_ids: Option<Vec<i32>>,
    pub min_price: Option<i64>,
    pub max_price: Option<i64>,
    pub end_before: Option<DateTime<Utc>>,
    pub seller_id: Option<Uuid>,
    pub status: Option<ListingStatus>,
    pub page: i64,
    pub page_size: i64,
}

#[async_trait]
pub trait ListingRepository: Send + Sync {
    async fn create_listing(
        &self,
        seller_id: Uuid,
        seller_name: String,
        category_id: Option<i32>,
        category_name: String,
        title: String,
        description: String,
        start_price: i64,
        reserve_price: Option<i64>,
        min_increment: i64,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    ) -> Result<Listing, ListingError>;

    async fn get_listing(
        &self,
        id: Uuid,
    ) -> Result<Option<Listing>, ListingError>;

    async fn list_listings(
        &self,
        filter: ListingFilter,
    ) -> Result<(Vec<Listing>, i64), ListingError>;

    async fn update_listing(
        &self,
        id: Uuid,
        description: Option<String>,
    ) -> Result<Listing, ListingError>;

    async fn cancel_listing(
        &self,
        id: Uuid,
    ) -> Result<(), ListingError>;
}