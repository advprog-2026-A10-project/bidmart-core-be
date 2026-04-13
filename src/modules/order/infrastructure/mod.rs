use std::sync::Arc;

use crate::modules::order::domain::entities::{
    Notification, NotificationChannel, NotificationType, Order, OrderStage, OrderStatus,
};
use crate::modules::order::infrastructure::repositories::{
    in_memory_notification_repository::InMemoryNotificationRepository,
    in_memory_order_repository::InMemoryOrderRepository,
};
use chrono::Utc;
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub order_repo: Arc<InMemoryOrderRepository>,
    pub notification_repo: Arc<InMemoryNotificationRepository>,
}

pub fn create_app_state(pool: PgPool) -> AppState {
    let sample_orders = sample_order_data();
    let order_repo = Arc::new(InMemoryOrderRepository::new(sample_orders));
    let notification_repo = Arc::new(InMemoryNotificationRepository::new(
        sample_notification_data(),
    ));

    AppState {
        pool,
        order_repo,
        notification_repo,
    }
}

fn sample_order_data() -> Vec<Order> {
    vec![
        Order {
            id: Uuid::new_v4(),
            lot: "Banksy - Shredded Beauty".into(),
            stage: OrderStage::Active,
            status: OrderStatus::AwaitingPayment,
            buyer_id: "buyer-vel".into(),
            seller_id: "seller-adr".into(),
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
            buyer_id: "buyer-krl".into(),
            seller_id: "seller-ddl".into(),
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
            metadata: None,
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
            metadata: None,
        },
    ]
}
