use std::sync::Arc;

use crate::modules::order::domain::entities::{
    Notification, NotificationChannel, NotificationType, Order, OrderStage, OrderStatus,
};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::domain::traits::{NotificationRepository, OrderRepository};
use crate::modules::order::infrastructure::repositories::{
    db_notification_repository::DbNotificationRepository, db_order_repository::DbOrderRepository,
    in_memory_notification_repository::InMemoryNotificationRepository,
    in_memory_order_repository::InMemoryOrderRepository,
};
use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use sqlx::postgres::PgPool;
use uuid::Uuid;

const SAMPLE_BUYER_ONE_ID: &str = "11111111-1111-1111-1111-111111111111";
const SAMPLE_SELLER_ONE_ID: &str = "22222222-2222-2222-2222-222222222222";
const SAMPLE_BUYER_TWO_ID: &str = "33333333-3333-3333-3333-333333333333";
const SAMPLE_SELLER_TWO_ID: &str = "44444444-4444-4444-4444-444444444444";

pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

#[derive(Clone)]
pub struct OrderRepositoryHandle {
    inner: Arc<dyn OrderRepository>,
}

impl OrderRepositoryHandle {
    fn new<T>(repository: T) -> Self
    where
        T: OrderRepository + 'static,
    {
        Self {
            inner: Arc::new(repository),
        }
    }
}

#[async_trait]
impl OrderRepository for OrderRepositoryHandle {
    async fn list_orders(
        &self,
        role: &str,
        user_id: Option<&str>,
        stage: Option<OrderStage>,
    ) -> Result<Vec<Order>, OrderError> {
        self.inner.list_orders(role, user_id, stage).await
    }

    async fn get_order(
        &self,
        order_id: crate::modules::order::domain::entities::OrderId,
    ) -> Result<Order, OrderError> {
        self.inner.get_order(order_id).await
    }

    async fn confirm_order(
        &self,
        order_id: crate::modules::order::domain::entities::OrderId,
        actor_id: &str,
    ) -> Result<(), OrderError> {
        self.inner.confirm_order(order_id, actor_id).await
    }

    async fn create_dispute(
        &self,
        order_id: crate::modules::order::domain::entities::OrderId,
        reporter_id: &str,
        reason: &str,
        details: Option<&str>,
    ) -> Result<(), OrderError> {
        self.inner
            .create_dispute(order_id, reporter_id, reason, details)
            .await
    }

    async fn update_shipping_status(
        &self,
        order_id: crate::modules::order::domain::entities::OrderId,
        status: &str,
        tracking: Option<&str>,
    ) -> Result<(), OrderError> {
        self.inner
            .update_shipping_status(order_id, status, tracking)
            .await
    }

    async fn record_event(
        &self,
        order_id: crate::modules::order::domain::entities::OrderId,
        event: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), OrderError> {
        self.inner.record_event(order_id, event, metadata).await
    }
}

#[derive(Clone)]
pub struct NotificationRepositoryHandle {
    inner: Arc<dyn NotificationRepository>,
}

impl NotificationRepositoryHandle {
    fn new<T>(repository: T) -> Self
    where
        T: NotificationRepository + 'static,
    {
        Self {
            inner: Arc::new(repository),
        }
    }
}

#[async_trait]
impl NotificationRepository for NotificationRepositoryHandle {
    async fn list_notifications(
        &self,
        user_id: Option<&str>,
        limit: Option<u32>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError> {
        self.inner
            .list_notifications(user_id, limit, unread_only)
            .await
    }

    async fn get_notification(
        &self,
        notification_id: crate::modules::order::domain::NotificationId,
    ) -> Result<Notification, NotificationError> {
        self.inner.get_notification(notification_id).await
    }

    async fn mark_as_read(
        &self,
        notification_id: crate::modules::order::domain::NotificationId,
        actor_id: &str,
    ) -> Result<(), NotificationError> {
        self.inner.mark_as_read(notification_id, actor_id).await
    }

    async fn publish_event(
        &self,
        payload: crate::modules::order::domain::entities::NotificationEventPayload,
    ) -> Result<(), NotificationError> {
        self.inner.publish_event(payload).await
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_base_url: String,
    pub auth_http_client: Client,
    pub order_repo: OrderRepositoryHandle,
    pub notification_repo: NotificationRepositoryHandle,
}

pub fn create_app_state(pool: PgPool) -> AppState {
    create_app_state_with_notification_repo(
        pool,
        OrderRepositoryHandle::new(Arc::new(InMemoryOrderRepository::new(sample_order_data()))),
        NotificationRepositoryHandle::new(Arc::new(InMemoryNotificationRepository::new(
            sample_notification_data(),
        ))),
    )
}

pub fn create_runtime_app_state(pool: PgPool) -> AppState {
    create_runtime_app_state_with_auth(pool.clone(), String::new())
}

pub fn create_runtime_app_state_with_auth(pool: PgPool, auth_base_url: String) -> AppState {
    create_app_state_with_repositories(
        pool.clone(),
        auth_base_url,
        OrderRepositoryHandle::new(DbOrderRepository::new(pool.clone())),
        NotificationRepositoryHandle::new(DbNotificationRepository::new(pool)),
    )
}

fn create_app_state_with_notification_repo(
    pool: PgPool,
    order_repo: OrderRepositoryHandle,
    notification_repo: NotificationRepositoryHandle,
) -> AppState {
    create_app_state_with_repositories(pool, String::new(), order_repo, notification_repo)
}

fn create_app_state_with_repositories(
    pool: PgPool,
    auth_base_url: String,
    order_repo: OrderRepositoryHandle,
    notification_repo: NotificationRepositoryHandle,
) -> AppState {
    AppState {
        pool,
        auth_base_url,
        auth_http_client: Client::new(),
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
