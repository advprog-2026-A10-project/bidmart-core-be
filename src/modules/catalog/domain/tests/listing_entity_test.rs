use chrono::{Duration, Utc};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::{Listing, ListingStatus};

fn make_listing(status: ListingStatus, bid_count: i32) -> Listing {
    Listing {
        id: Uuid::new_v4(),
        seller_id: Uuid::new_v4(),
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

#[test]
fn has_bids_returns_false_when_bid_count_zero() {
    assert!(!make_listing(ListingStatus::Active, 0).has_bids());
}

#[test]
fn has_bids_returns_true_when_bid_count_positive() {
    assert!(make_listing(ListingStatus::Active, 3).has_bids());
}

#[test]
fn is_mutable_returns_true_for_draft_without_bids() {
    assert!(make_listing(ListingStatus::Draft, 0).is_mutable_by_seller());
}

#[test]
fn is_mutable_returns_true_for_active_without_bids() {
    assert!(make_listing(ListingStatus::Active, 0).is_mutable_by_seller());
}

#[test]
fn is_mutable_returns_false_for_active_with_bids() {
    assert!(!make_listing(ListingStatus::Active, 1).is_mutable_by_seller());
}

#[test]
fn is_mutable_returns_false_for_sold_listing() {
    assert!(!make_listing(ListingStatus::Sold, 0).is_mutable_by_seller());
}

#[test]
fn is_mutable_returns_false_for_cancelled_listing() {
    assert!(!make_listing(ListingStatus::Cancelled, 0).is_mutable_by_seller());
}

#[test]
fn is_mutable_returns_false_for_expired_listing() {
    assert!(!make_listing(ListingStatus::Expired, 0).is_mutable_by_seller());
}
