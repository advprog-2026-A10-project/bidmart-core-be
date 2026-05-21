//! Test-only helpers for the order module.
//!
//! Sample fixtures and the in-memory `AppState` builder live here so the
//! production binary does not carry seed data. Gated by `#[cfg(test)]` via
//! `mod test_support` in `infrastructure/mod.rs`.

use std::sync::Arc;

use chrono::Utc;
use serde_json::json;
use sqlx::postgres::PgPool;
use uuid::Uuid;

use crate::modules::order::domain::entities::{
    Notification, NotificationChannel, NotificationType, Order, OrderStage, OrderStatus,
};
use crate::modules::order::infrastructure::repositories::{
    in_memory_notification_repository::InMemoryNotificationRepository,
    in_memory_order_repository::InMemoryOrderRepository,
};
use crate::modules::order::infrastructure::{
    AppState, NotificationRepositoryHandle, OrderRepositoryHandle,
};

pub const SAMPLE_BUYER_ONE_ID: &str = "11111111-1111-1111-1111-111111111111";
pub const SAMPLE_SELLER_ONE_ID: &str = "22222222-2222-2222-2222-222222222222";
pub const SAMPLE_BUYER_TWO_ID: &str = "33333333-3333-3333-3333-333333333333";
pub const SAMPLE_SELLER_TWO_ID: &str = "44444444-4444-4444-4444-444444444444";

pub fn create_app_state(pool: PgPool) -> AppState {
    AppState {
        pool,
        auth_base_url: String::new(),
        auth_http_client: reqwest::Client::new(),
        order_repo: OrderRepositoryHandle::new(Arc::new(InMemoryOrderRepository::new(
            sample_order_data(),
        ))),
        notification_repo: NotificationRepositoryHandle::new(Arc::new(
            InMemoryNotificationRepository::new(sample_notification_data()),
        )),
        internal_secret: None,
    }
}

fn sample_order_data() -> Vec<Order> {
    vec![
        Order {
            id: Uuid::new_v4(),
            lot: "Banksy - Shredded Beauty".into(),
            stage: OrderStage::Active,
            status: OrderStatus::AwaitingPayment,
            buyer_id: SAMPLE_BUYER_ONE_ID.into(),
            seller_id: SAMPLE_SELLER_ONE_ID.into(),
            total: "$25,400,000".into(),
            currency: "USD".into(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            tags: vec!["Escrow pending".into(), "Anti-sniping".into()],
            last_activity: "Just now".into(),
        },
        Order {
            id: Uuid::new_v4(),
            lot: "Banksy - Flower Thrower".into(),
            stage: OrderStage::Processing,
            status: OrderStatus::InTransit,
            buyer_id: SAMPLE_BUYER_TWO_ID.into(),
            seller_id: SAMPLE_SELLER_TWO_ID.into(),
            total: "$1,250,000".into(),
            currency: "USD".into(),
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
            tags: vec!["Courier: FedEx".into(), "Signature required".into()],
            last_activity: "6 minutes ago".into(),
        },
    ]
}

fn sample_notification_data() -> Vec<Notification> {
    vec![
        Notification {
            id: Uuid::new_v4(),
            order_id: None,
            title: "BidPlaced · Banksy - Shredded Beauty".into(),
            body: "VEL placed a live bid and extended the clock.".into(),
            channel: NotificationChannel::Email,
            notification_type: NotificationType::BidPlaced,
            created_at: Utc::now().to_rfc3339(),
            read_at: None,
            metadata: Some(json!({ "userId": SAMPLE_BUYER_ONE_ID })),
        },
        Notification {
            id: Uuid::new_v4(),
            order_id: None,
            title: "WinnerDetermined · Banksy - Flower Thrower".into(),
            body: "KRL won, funds reserved, order created.".into(),
            channel: NotificationChannel::Inbox,
            notification_type: NotificationType::WinnerDetermined,
            created_at: Utc::now().to_rfc3339(),
            read_at: None,
            metadata: Some(json!({ "userId": SAMPLE_BUYER_TWO_ID })),
        },
    ]
}
