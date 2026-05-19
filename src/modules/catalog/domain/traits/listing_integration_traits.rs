use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::domain::errors::ListingError;

#[async_trait]
pub trait ListingIntegrationPort: Send + Sync {
    async fn get_listing_status(&self, id: Uuid) -> Result<Option<ListingStatus>, ListingError>;

    async fn update_listing_status(
        &self,
        id: Uuid,
        status: ListingStatus,
    ) -> Result<(), ListingError>;

    async fn update_current_price(&self, id: Uuid, new_price: i64) -> Result<(), ListingError>;

    async fn increment_bid_count(&self, id: Uuid) -> Result<(), ListingError>;

    async fn link_auction(&self, listing_id: Uuid, auction_id: Uuid) -> Result<(), ListingError>;

    async fn update_ends_at(
        &self,
        listing_id: Uuid,
        new_ends_at: DateTime<Utc>,
    ) -> Result<(), ListingError>;
}
