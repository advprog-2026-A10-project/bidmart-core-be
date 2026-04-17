use std::sync::Arc;

use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::{
    CategoryRepository, ListingFilter, ListingImageRepository, ListingRepository,
};
use crate::modules::catalog::application::dto::listing_dto::{
    ListingImageResponse, PaginatedResponse,
};
use crate::modules::catalog::application::dto::buyer_listing_dto::{
    BuyerListingDetailResponse, BuyerListingResponse, PublicListingQueryParams,
};

pub struct BuyerListingUseCases {
    listing_repo: Arc<dyn ListingRepository>,
    image_repo: Arc<dyn ListingImageRepository>,
    category_repo: Arc<dyn CategoryRepository>,
}

impl BuyerListingUseCases {
    pub fn new(
        listing_repo: Arc<dyn ListingRepository>,
        image_repo: Arc<dyn ListingImageRepository>,
        category_repo: Arc<dyn CategoryRepository>,
    ) -> Self {
        Self { listing_repo, image_repo, category_repo }
    }

    pub async fn browse_catalog(
        &self,
        params: PublicListingQueryParams,
    ) -> Result<PaginatedResponse<BuyerListingResponse>, ListingError> {
        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let filter = ListingFilter {
            keyword: params.q,
            category_ids: params.category_id.map(|id| vec![id]),
            min_price: params.min_price,
            max_price: params.max_price,
            end_before: params.end_before,
            seller_id: None,
            status: Some(ListingStatus::Active),
            page,
            page_size,
        };

        let (listings, total) = self.listing_repo.list_listings(filter).await?;

        Ok(PaginatedResponse {
            data: listings.into_iter().map(BuyerListingResponse::from).collect(),
            total,
            page,
            page_size,
        })
    }

    pub async fn get_public_listing(
        &self,
        listing_id: Uuid,
    ) -> Result<BuyerListingDetailResponse, ListingError> {
        let listing = self
            .listing_repo
            .get_listing(listing_id)
            .await?
            .ok_or(ListingError::NotFound)?;

        if listing.status != ListingStatus::Active {
            return Err(ListingError::NotFound);
        }

        let images = self.image_repo.get_listing_images(listing_id).await?;

        Ok(BuyerListingDetailResponse {
            listing: BuyerListingResponse::from(listing),
            images: images.into_iter().map(ListingImageResponse::from).collect(),
        })
    }

    pub async fn browse_by_category_slug(
        &self,
        slug: String,
        page: i64,
        page_size: i64,
    ) -> Result<PaginatedResponse<BuyerListingResponse>, ListingError> {
        let category = self
            .category_repo
            .get_category_by_slug(&slug)
            .await
            .map_err(|e| ListingError::InternalError(e.to_string()))?
            .ok_or(ListingError::NotFound)?;

        let subtree_ids = self
            .category_repo
            .get_subtree_ids(category.id)
            .await
            .map_err(|e| ListingError::InternalError(e.to_string()))?;

        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);

        let filter = ListingFilter {
            keyword: None,
            category_ids: Some(subtree_ids),
            min_price: None,
            max_price: None,
            end_before: None,
            seller_id: None,
            status: Some(ListingStatus::Active),
            page,
            page_size,
        };

        let (listings, total) = self.listing_repo.list_listings(filter).await?;

        Ok(PaginatedResponse {
            data: listings.into_iter().map(BuyerListingResponse::from).collect(),
            total,
            page,
            page_size,
        })
    }
}
