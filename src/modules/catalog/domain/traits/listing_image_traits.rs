use async_trait::async_trait;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingImage;
use crate::modules::catalog::domain::errors::ListingError;

#[async_trait]
pub trait ListingImageRepository: Send + Sync {
    async fn get_listing_images(
        &self,
        listing_id: Uuid,
    ) -> Result<Vec<ListingImage>, ListingError>;

    async fn replace_listing_images(
        &self,
        listing_id: Uuid,
        urls: Vec<String>,
    ) -> Result<Vec<ListingImage>, ListingError>;
}