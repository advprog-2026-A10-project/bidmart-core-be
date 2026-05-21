use std::sync::Arc;

use crate::modules::order::domain::entities::{Notification, Order, OrderStage};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::domain::traits::{NotificationRepository, OrderRepository};
use crate::modules::order::infrastructure::repositories::{
    db_notification_repository::DbNotificationRepository, db_order_repository::DbOrderRepository,
};
use async_trait::async_trait;
use reqwest::Client;
use sqlx::postgres::PgPool;

#[cfg(test)]
pub mod test_support;

pub mod controllers;
pub mod middleware;
pub mod repositories;
pub mod services;

#[derive(Clone)]
pub struct OrderRepositoryHandle {
    inner: Arc<dyn OrderRepository>,
}

impl OrderRepositoryHandle {
    pub fn new<T>(repository: T) -> Self
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
    pub fn new<T>(repository: T) -> Self
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
    /// Shared secret required on the `X-Internal-Secret` header for every
    /// `/events/*` request. `None` disables the check (dev mode); production
    /// deployments must set `APP_ORDER_INTERNAL_SECRET`.
    pub internal_secret: Option<Arc<str>>,
}

pub fn create_runtime_app_state_with_auth(pool: PgPool, auth_base_url: String) -> AppState {
    let internal_secret = std::env::var("APP_ORDER_INTERNAL_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(Arc::<str>::from);

    AppState {
        pool: pool.clone(),
        auth_base_url,
        auth_http_client: Client::new(),
        order_repo: OrderRepositoryHandle::new(DbOrderRepository::new(pool.clone())),
        notification_repo: NotificationRepositoryHandle::new(DbNotificationRepository::new(pool)),
        internal_secret,
    }
}
