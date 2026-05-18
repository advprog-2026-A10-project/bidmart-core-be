use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::ListingIntegrationPort;

pub struct ListingIntegrationService {
    repo: Arc<dyn ListingIntegrationPort>,
}

impl ListingIntegrationService {
    pub fn new(repo: Arc<dyn ListingIntegrationPort>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl ListingIntegrationPort for ListingIntegrationService {
    async fn get_listing_status(
        &self,
        id: Uuid,
    ) -> Result<Option<ListingStatus>, ListingError> {
        self.repo.get_listing_status(id).await
    }

    async fn update_listing_status(
        &self,
        id: Uuid,
        status: ListingStatus,
    ) -> Result<(), ListingError> {
        self.repo.update_listing_status(id, status).await
    }

    async fn link_auction(
        &self,
        listing_id: Uuid,
        auction_id: Uuid,
    ) -> Result<(), ListingError> {
        self.repo.link_auction(listing_id, auction_id).await
    }

    async fn update_ends_at(
        &self,
        listing_id: Uuid,
        new_ends_at: DateTime<Utc>,
    ) -> Result<(), ListingError> {
        self.repo.update_ends_at(listing_id, new_ends_at).await
    }

    async fn update_current_price(
        &self,
        id: Uuid,
        new_price: i64,
    ) -> Result<(), ListingError> {
        self.repo.update_current_price(id, new_price).await
    }

    async fn increment_bid_count(&self, id: Uuid) -> Result<(), ListingError> {
        self.repo.increment_bid_count(id).await
    }
}
