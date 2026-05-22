use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use uuid::Uuid;

use crate::modules::catalog::application::dto::listing_dto::{
    CreateListingRequest, UpdateListingRequest,
};
use crate::modules::catalog::application::use_cases::listing_use_cases::ListingUseCases;
use crate::modules::catalog::domain::entities::{Category, Listing, ListingImage, ListingStatus};
use crate::modules::catalog::domain::errors::{CategoryError, ListingError};
use crate::modules::catalog::domain::traits::{
    AuctionLifecyclePort, CategoryRepository, ListingFilter, ListingImageRepository,
    ListingRepository,
};

fn make_listing(status: ListingStatus, bid_count: i32, seller_id: Uuid) -> Listing {
    Listing {
        id: Uuid::new_v4(),
        seller_id,
        seller_name: "Seller".to_string(),
        category_id: None,
        category_name: String::new(),
        title: "Test Item".to_string(),
        description: String::new(),
        start_price: 1000,
        reserve_price: None,
        current_price: 1000,
        min_increment: 100,
        bid_count,
        status,
        auction_id: None,
        starts_at: Utc::now(),
        ends_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn make_category(child_count: i32) -> Category {
    Category {
        id: 1,
        parent_id: None,
        name: "Electronics".to_string(),
        slug: "electronics".to_string(),
        image_url: None,
        child_count,
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
    ) -> Result<Listing, ListingError> {
        Ok(Listing {
            id: Uuid::new_v4(),
            seller_id,
            seller_name,
            category_id,
            category_name,
            title,
            description,
            start_price,
            reserve_price,
            current_price: start_price,
            min_increment,
            bid_count: 0,
            status: ListingStatus::Draft,
            auction_id: None,
            starts_at,
            ends_at,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    async fn get_listing(&self, _: Uuid) -> Result<Option<Listing>, ListingError> {
        Ok(self.listing.clone())
    }
    async fn list_listings(&self, _: ListingFilter) -> Result<(Vec<Listing>, i64), ListingError> {
        Ok((Vec::new(), 0))
    }
    async fn update_listing(
        &self,
        _: Uuid,
        description: Option<String>,
    ) -> Result<Listing, ListingError> {
        let mut l = self.listing.clone().unwrap();
        if let Some(d) = description {
            l.description = d;
        }
        Ok(l)
    }
    async fn cancel_listing(&self, _: Uuid) -> Result<(), ListingError> {
        Ok(())
    }
    async fn publish_listing(&self, _: Uuid) -> Result<Listing, ListingError> {
        let mut l = self.listing.clone().unwrap();
        l.status = ListingStatus::Active;
        Ok(l)
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
        listing_id: Uuid,
        urls: Vec<String>,
    ) -> Result<Vec<ListingImage>, ListingError> {
        Ok(urls
            .into_iter()
            .enumerate()
            .map(|(i, url)| ListingImage {
                id: Uuid::new_v4(),
                listing_id,
                url,
                order: i as i32,
                created_at: Utc::now(),
            })
            .collect())
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
        Ok(self.category.iter().cloned().collect())
    }
    async fn create_category(
        &self,
        name: String,
        slug: String,
        parent_id: Option<i32>,
        image_url: Option<String>,
    ) -> Result<Category, CategoryError> {
        Ok(Category {
            id: 99,
            parent_id,
            name,
            slug,
            image_url,
            child_count: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    async fn update_category(
        &self,
        _: i32,
        _: Option<String>,
        _: Option<String>,
        _: Option<String>,
    ) -> Result<Category, CategoryError> {
        Ok(self.category.clone().unwrap())
    }
    async fn delete_category(&self, _: i32) -> Result<(), CategoryError> {
        Ok(())
    }
}

struct MockAuctionLifecycle;

#[async_trait]
impl AuctionLifecyclePort for MockAuctionLifecycle {
    async fn start_auction_for_listing(
        &self,
        _: &Listing,
        _: Option<String>,
    ) -> Result<Uuid, ListingError> {
        Ok(Uuid::new_v4())
    }
}

fn make_uc(listing: Option<Listing>, category: Option<Category>) -> ListingUseCases {
    ListingUseCases::new(
        Arc::new(MockListingRepo { listing }),
        Arc::new(MockImageRepo),
        Arc::new(MockCategoryRepo { category }),
        Arc::new(MockAuctionLifecycle),
    )
}

fn future(hours: i64) -> DateTime<Utc> {
    Utc::now() + Duration::hours(hours)
}

fn valid_create_request() -> CreateListingRequest {
    CreateListingRequest {
        title: "My Item".to_string(),
        description: "desc".to_string(),
        start_price: 1000,
        reserve_price: None,
        min_increment: 100,
        category_id: None,
        image_urls: vec![],
        starts_at: future(1),
        ends_at: future(24),
    }
}

// create_listing

#[tokio::test]
async fn create_listing_empty_title_returns_validation_error() {
    let req = CreateListingRequest { title: "   ".to_string(), ..valid_create_request() };
    assert!(matches!(
        make_uc(None, None)
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_non_positive_start_price_returns_validation_error() {
    let req = CreateListingRequest { start_price: 0, ..valid_create_request() };
    assert!(matches!(
        make_uc(None, None)
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_reserve_below_start_price_returns_validation_error() {
    let req = CreateListingRequest {
        start_price: 1000,
        reserve_price: Some(500),
        ..valid_create_request()
    };
    assert!(matches!(
        make_uc(None, None)
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_ends_before_starts_returns_validation_error() {
    let base = future(10);
    let req = CreateListingRequest {
        starts_at: base,
        ends_at: base - Duration::hours(1),
        ..valid_create_request()
    };
    assert!(matches!(
        make_uc(None, None)
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_unknown_category_returns_validation_error() {
    let req = CreateListingRequest { category_id: Some(99), ..valid_create_request() };
    assert!(matches!(
        make_uc(None, None)
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_non_leaf_category_returns_validation_error() {
    let req = CreateListingRequest { category_id: Some(1), ..valid_create_request() };
    assert!(matches!(
        make_uc(None, Some(make_category(2)))
            .create_listing(Uuid::new_v4(), "S".to_string(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn create_listing_with_valid_data_succeeds() {
    assert!(make_uc(None, None)
        .create_listing(Uuid::new_v4(), "Seller".to_string(), valid_create_request())
        .await
        .is_ok());
}

// get_my_listing

#[tokio::test]
async fn get_my_listing_returns_unauthorized_when_wrong_seller() {
    let listing = make_listing(ListingStatus::Draft, 0, Uuid::new_v4());
    assert!(matches!(
        make_uc(Some(listing), None)
            .get_my_listing(Uuid::new_v4(), Uuid::new_v4())
            .await
            .unwrap_err(),
        ListingError::Unauthorized
    ));
}

#[tokio::test]
async fn get_my_listing_returns_detail_when_owner_requests() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Draft, 0, seller_id);
    assert!(make_uc(Some(listing), None)
        .get_my_listing(seller_id, Uuid::new_v4())
        .await
        .is_ok());
}

// update_listing

#[tokio::test]
async fn update_listing_returns_not_editable_when_has_bids() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Active, 1, seller_id);
    let req = UpdateListingRequest { description: Some("new".to_string()), image_urls: None };
    assert!(matches!(
        make_uc(Some(listing), None)
            .update_listing(seller_id, Uuid::new_v4(), req)
            .await
            .unwrap_err(),
        ListingError::NotEditable
    ));
}

#[tokio::test]
async fn update_listing_rejects_invalid_image_url() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Draft, 0, seller_id);
    let req = UpdateListingRequest {
        description: None,
        image_urls: Some(vec!["not-a-url".to_string()]),
    };
    assert!(matches!(
        make_uc(Some(listing), None)
            .update_listing(seller_id, Uuid::new_v4(), req)
            .await
            .unwrap_err(),
        ListingError::ValidationError(_)
    ));
}

#[tokio::test]
async fn update_listing_with_valid_data_succeeds() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Draft, 0, seller_id);
    let req = UpdateListingRequest { description: Some("updated".to_string()), image_urls: None };
    assert!(make_uc(Some(listing), None)
        .update_listing(seller_id, Uuid::new_v4(), req)
        .await
        .is_ok());
}

// cancel_listing

#[tokio::test]
async fn cancel_listing_returns_not_cancellable_when_has_bids() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Active, 1, seller_id);
    assert!(matches!(
        make_uc(Some(listing), None)
            .cancel_listing(seller_id, Uuid::new_v4())
            .await
            .unwrap_err(),
        ListingError::NotCancellable
    ));
}

#[tokio::test]
async fn cancel_draft_listing_succeeds() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Draft, 0, seller_id);
    assert!(make_uc(Some(listing), None)
        .cancel_listing(seller_id, Uuid::new_v4())
        .await
        .is_ok());
}

// publish_listing

#[tokio::test]
async fn publish_draft_listing_succeeds() {
    let seller_id = Uuid::new_v4();
    let listing = make_listing(ListingStatus::Draft, 0, seller_id);
    assert!(make_uc(Some(listing), None)
        .publish_listing(seller_id, Uuid::new_v4())
        .await
        .is_ok());
}
