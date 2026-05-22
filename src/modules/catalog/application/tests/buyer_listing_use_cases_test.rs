use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::catalog::application::use_cases::buyer_listing_use_cases::BuyerListingUseCases;
use crate::modules::catalog::domain::entities::{Category, Listing, ListingImage, ListingStatus};
use crate::modules::catalog::domain::errors::{CategoryError, ListingError};
use crate::modules::catalog::domain::traits::{
    CategoryRepository, ListingFilter, ListingImageRepository, ListingRepository,
};

fn make_listing(status: ListingStatus) -> Listing {
    Listing {
        id: Uuid::new_v4(),
        seller_id: Uuid::new_v4(),
        seller_name: "Seller".to_string(),
        category_id: None,
        category_name: String::new(),
        title: "Item".to_string(),
        description: String::new(),
        start_price: 1000,
        reserve_price: None,
        current_price: 1000,
        min_increment: 100,
        bid_count: 0,
        status,
        auction_id: None,
        starts_at: Utc::now(),
        ends_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

struct MockListingRepo {
    listing: Option<Listing>,
}

#[async_trait]
impl ListingRepository for MockListingRepo {
    async fn create_listing(
        &self,
        _: Uuid,
        _: String,
        _: Option<i32>,
        _: String,
        _: String,
        _: String,
        _: i64,
        _: Option<i64>,
        _: i64,
        _: DateTime<Utc>,
        _: DateTime<Utc>,
    ) -> Result<Listing, ListingError> {
        unimplemented!()
    }
    async fn get_listing(&self, _: Uuid) -> Result<Option<Listing>, ListingError> {
        Ok(self.listing.clone())
    }
    async fn list_listings(&self, _: ListingFilter) -> Result<(Vec<Listing>, i64), ListingError> {
        Ok((Vec::new(), 0))
    }
    async fn update_listing(&self, _: Uuid, _: Option<String>) -> Result<Listing, ListingError> {
        unimplemented!()
    }
    async fn cancel_listing(&self, _: Uuid) -> Result<(), ListingError> {
        unimplemented!()
    }
    async fn publish_listing(&self, _: Uuid) -> Result<Listing, ListingError> {
        unimplemented!()
    }
}

struct MockImageRepo;

#[async_trait]
impl ListingImageRepository for MockImageRepo {
    async fn get_listing_images(&self, _: Uuid) -> Result<Vec<ListingImage>, ListingError> {
        Ok(Vec::new())
    }
    async fn replace_listing_images(
        &self,
        _: Uuid,
        _: Vec<String>,
    ) -> Result<Vec<ListingImage>, ListingError> {
        Ok(Vec::new())
    }
}

struct MockCategoryRepo {
    category: Option<Category>,
}

#[async_trait]
impl CategoryRepository for MockCategoryRepo {
    async fn get_category(&self, _: i32) -> Result<Option<Category>, CategoryError> {
        Ok(self.category.clone())
    }
    async fn get_category_by_slug(&self, _: &str) -> Result<Option<Category>, CategoryError> {
        Ok(self.category.clone())
    }
    async fn resolve_category_path(
        &self,
        _: &[String],
    ) -> Result<Option<Category>, CategoryError> {
        Ok(self.category.clone())
    }
    async fn get_subtree_ids(&self, root_id: i32) -> Result<Vec<i32>, CategoryError> {
        Ok(vec![root_id])
    }
    async fn list_categories(&self, _: Option<i32>) -> Result<Vec<Category>, CategoryError> {
        Ok(Vec::new())
    }
    async fn create_category(
        &self,
        _: String,
        _: String,
        _: Option<i32>,
        _: Option<String>,
    ) -> Result<Category, CategoryError> {
        unimplemented!()
    }
    async fn update_category(
        &self,
        _: i32,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
    ) -> Result<Category, CategoryError> {
        unimplemented!()
    }
    async fn delete_category(&self, _: i32) -> Result<(), CategoryError> {
        unimplemented!()
    }
}

fn make_uc(listing: Option<Listing>, category: Option<Category>) -> BuyerListingUseCases {
    BuyerListingUseCases::new(
        Arc::new(MockListingRepo { listing }),
        Arc::new(MockImageRepo),
        Arc::new(MockCategoryRepo { category }),
    )
}

// get_public_listing

#[tokio::test]
async fn get_public_listing_returns_not_found_when_missing() {
    assert!(matches!(
        make_uc(None, None)
            .get_public_listing(Uuid::new_v4())
            .await
            .unwrap_err(),
        ListingError::NotFound
    ));
}

#[tokio::test]
async fn get_public_listing_returns_not_found_for_non_active_status() {
    assert!(matches!(
        make_uc(Some(make_listing(ListingStatus::Draft)), None)
            .get_public_listing(Uuid::new_v4())
            .await
            .unwrap_err(),
        ListingError::NotFound
    ));
}

#[tokio::test]
async fn get_public_listing_returns_detail_when_active() {
    assert!(make_uc(Some(make_listing(ListingStatus::Active)), None)
        .get_public_listing(Uuid::new_v4())
        .await
        .is_ok());
}

// browse_by_category_path

#[tokio::test]
async fn browse_by_category_path_empty_returns_validation_error() {
    assert!(matches!(
        make_uc(None, None)
            .browse_by_category_path("", 1, 20)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn browse_by_category_path_over_50_segments_returns_validation_error() {
    let path = (0..51).map(|i| format!("seg{}", i)).collect::<Vec<_>>().join("/");
    assert!(matches!(
        make_uc(None, None)
            .browse_by_category_path(&path, 1, 20)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn browse_by_category_path_returns_not_found_when_category_missing() {
    assert!(matches!(
        make_uc(None, None)
            .browse_by_category_path("electronics", 1, 20)
            .await
            .unwrap_err(),
        ListingError::NotFound
    ));
}

#[tokio::test]
async fn browse_by_category_path_with_valid_path_succeeds() {
    let category = Category {
        id: 1,
        parent_id: None,
        name: "Electronics".to_string(),
        slug: "electronics".to_string(),
        image_url: None,
        child_count: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(make_uc(None, Some(category))
        .browse_by_category_path("electronics", 1, 20)
        .await
        .is_ok());
}
