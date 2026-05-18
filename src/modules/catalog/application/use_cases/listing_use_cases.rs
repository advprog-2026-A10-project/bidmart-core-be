use std::sync::Arc;

use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::{
    CategoryRepository, ListingFilter, ListingImageRepository, ListingRepository,
};

use crate::modules::catalog::application::dto::listing_dto::{
    CreateListingRequest, ListingDetailResponse, ListingImageResponse,
    ListingQueryParams, ListingResponse, PaginatedResponse, UpdateListingRequest,
};

const MAX_IMAGES: usize = 10;
const MAX_IMAGE_URL_LEN: usize = 2048;
const CLOCK_SKEW: Duration = Duration::minutes(5);

fn validate_image_urls(urls: &[String]) -> Result<(), ListingError> {
    if urls.len() > MAX_IMAGES {
        return Err(ListingError::ValidationError(format!(
            "At most {MAX_IMAGES} images are allowed"
        )));
    }
    for url in urls {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(ListingError::ValidationError(
                "Image URL cannot be empty".to_string(),
            ));
        }
        if trimmed.len() > MAX_IMAGE_URL_LEN {
            return Err(ListingError::ValidationError(
                "Image URL is too long".to_string(),
            ));
        }
        if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
            return Err(ListingError::ValidationError(
                "Image URL must start with http:// or https://".to_string(),
            ));
        }
    }
    Ok(())
}

pub struct ListingUseCases {
    listing_repo: Arc<dyn ListingRepository>,
    image_repo: Arc<dyn ListingImageRepository>,
    category_repo: Arc<dyn CategoryRepository>,
}

impl ListingUseCases {
    pub fn new(
        listing_repo: Arc<dyn ListingRepository>,
        image_repo: Arc<dyn ListingImageRepository>,
        category_repo: Arc<dyn CategoryRepository>,
    ) -> Self {
        Self { listing_repo, image_repo, category_repo }
    }

    pub async fn create_listing(
        &self,
        seller_id: Uuid,
        seller_name: String,
        req: CreateListingRequest,
    ) -> Result<ListingDetailResponse, ListingError> {
        if req.title.trim().is_empty() {
            return Err(ListingError::ValidationError("Title cannot be empty".to_string()));
        }
        if req.start_price <= 0 {
            return Err(ListingError::ValidationError(
                "Start price must be greater than zero".to_string(),
            ));
        }
        if req.min_increment <= 0 {
            return Err(ListingError::ValidationError(
                "Min increment must be greater than zero".to_string(),
            ));
        }
        if req.ends_at <= req.starts_at {
            return Err(ListingError::ValidationError(
                "ends_at must be after starts_at".to_string(),
            ));
        }
        if req.ends_at <= Utc::now() {
            return Err(ListingError::ValidationError(
                "ends_at must be in the future".to_string(),
            ));
        }
        if req.starts_at < Utc::now() - CLOCK_SKEW {
            return Err(ListingError::ValidationError(
                "starts_at cannot be in the past".to_string(),
            ));
        }
        if let Some(reserve) = req.reserve_price {
            if reserve < req.start_price {
                return Err(ListingError::ValidationError(
                    "Reserve price must be at least the start price".to_string(),
                ));
            }
        }
        validate_image_urls(&req.image_urls)?;

        // Resolve category name from category_id
        let (category_id, category_name) = match req.category_id {
            Some(id) => {
                let cat = self
                    .category_repo
                    .get_category(id)
                    .await
                    .map_err(|e| ListingError::InternalError(e.to_string()))?
                    .ok_or_else(|| {
                        ListingError::ValidationError("Category not found".to_string())
                    })?;
                if cat.child_count > 0 {
                    return Err(ListingError::ValidationError(
                        "Category must be a leaf (no subcategories)".to_string(),
                    ));
                }
                (Some(id), cat.name)
            }
            None => (None, String::new()),
        };

        let listing = self
            .listing_repo
            .create_listing(
                seller_id,
                seller_name,
                category_id,
                category_name,
                req.title,
                req.description,
                req.start_price,
                req.reserve_price,
                req.min_increment,
                req.starts_at,
                req.ends_at,
            )
            .await?;

        let images = if !req.image_urls.is_empty() {
            self.image_repo
                .replace_listing_images(listing.id, req.image_urls)
                .await?
        } else {
            Vec::new()
        };

        Ok(ListingDetailResponse {
            listing: ListingResponse::from(listing),
            images: images.into_iter().map(ListingImageResponse::from).collect(),
        })
    }

    pub async fn list_my_listings(
        &self,
        seller_id: Uuid,
        params: ListingQueryParams,
    ) -> Result<PaginatedResponse<ListingResponse>, ListingError> {
        let page = params.page.unwrap_or(1).max(1);
        let page_size = params.page_size.unwrap_or(20).clamp(1, 100);

        let filter = ListingFilter {
            keyword: None,
            category_ids: None,
            min_price: None,
            max_price: None,
            end_before: None,
            seller_id: Some(seller_id),
            status: None, // seller sees all their own statuses
            page,
            page_size,
        };

        let (listings, total) = self.listing_repo.list_listings(filter).await?;

        Ok(PaginatedResponse {
            data: listings.into_iter().map(ListingResponse::from).collect(),
            total,
            page,
            page_size,
        })
    }
    
    pub async fn get_my_listing(
        &self,
        seller_id: Uuid,
        listing_id: Uuid,
    ) -> Result<ListingDetailResponse, ListingError> {
        let listing = self
            .listing_repo
            .get_listing(listing_id)
            .await?
            .ok_or(ListingError::NotFound)?;

        if listing.seller_id != seller_id {
            return Err(ListingError::Unauthorized);
        }

        let images = self.image_repo.get_listing_images(listing_id).await?;

        Ok(ListingDetailResponse {
            listing: ListingResponse::from(listing),
            images: images.into_iter().map(ListingImageResponse::from).collect(),
        })
    }

    pub async fn update_listing(
        &self,
        seller_id: Uuid,
        listing_id: Uuid,
        req: UpdateListingRequest,
    ) -> Result<ListingDetailResponse, ListingError> {
        let listing = self
            .listing_repo
            .get_listing(listing_id)
            .await?
            .ok_or(ListingError::NotFound)?;

        if listing.seller_id != seller_id {
            return Err(ListingError::Unauthorized);
        }
        if !listing.is_mutable_by_seller() {
            return Err(ListingError::NotEditable);
        }

        if let Some(urls) = req.image_urls.as_ref() {
            validate_image_urls(urls)?;
        }

        let updated = self
            .listing_repo
            .update_listing(listing_id, req.description)
            .await?;

        let images = match req.image_urls {
            Some(urls) => {
                self.image_repo
                    .replace_listing_images(listing_id, urls)
                    .await?
            }
            None => self.image_repo.get_listing_images(listing_id).await?,
        };

        Ok(ListingDetailResponse {
            listing: ListingResponse::from(updated),
            images: images.into_iter().map(ListingImageResponse::from).collect(),
        })
    }

    pub async fn cancel_listing(
        &self,
        seller_id: Uuid,
        listing_id: Uuid,
    ) -> Result<(), ListingError> {
        let listing = self
            .listing_repo
            .get_listing(listing_id)
            .await?
            .ok_or(ListingError::NotFound)?;

        if listing.seller_id != seller_id {
            return Err(ListingError::Unauthorized);
        }
        if !listing.is_mutable_by_seller() {
            return Err(ListingError::NotCancellable);
        }

        self.listing_repo.cancel_listing(listing_id).await
    }

    pub async fn publish_listing(
        &self,
        seller_id: Uuid,
        listing_id: Uuid,
    ) -> Result<ListingDetailResponse, ListingError> {
        let listing = self
            .listing_repo
            .get_listing(listing_id)
            .await?
            .ok_or(ListingError::NotFound)?;

        if listing.seller_id != seller_id {
            return Err(ListingError::Unauthorized);
        }
        if listing.status != ListingStatus::Draft {
            return Err(ListingError::NotPublishable);
        }

        let published = self.listing_repo.publish_listing(listing_id).await?;
        let images = self.image_repo.get_listing_images(listing_id).await?;

        Ok(ListingDetailResponse {
            listing: ListingResponse::from(published),
            images: images.into_iter().map(ListingImageResponse::from).collect(),
        })
    }
}
