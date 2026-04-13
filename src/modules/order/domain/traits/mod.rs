use async_trait::async_trait;

use crate::modules::order::domain::entities::{
    Notification, NotificationEventPayload, Order, OrderId, OrderStage,
};
use crate::modules::order::domain::errors::{NotificationError, OrderError};
use crate::modules::order::domain::NotificationId;

#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn list_orders(
        &self,
        role: &str,
        user_id: Option<&str>,
        stage: Option<OrderStage>,
    ) -> Result<Vec<Order>, OrderError>;

    async fn get_order(&self, order_id: OrderId) -> Result<Order, OrderError>;

    async fn confirm_order(&self, order_id: OrderId, actor_id: &str) -> Result<(), OrderError>;

    async fn create_dispute(
        &self,
        order_id: OrderId,
        reporter_id: &str,
        reason: &str,
    ) -> Result<(), OrderError>;

    async fn update_shipping_status(
        &self,
        order_id: OrderId,
        status: &str,
        tracking: Option<&str>,
    ) -> Result<(), OrderError>;

    async fn record_event(
        &self,
        order_id: OrderId,
        event: &str,
        metadata: Option<serde_json::Value>,
    ) -> Result<(), OrderError>;
}

#[async_trait]
pub trait NotificationRepository: Send + Sync {
    async fn list_notifications(
        &self,
        user_id: Option<&str>,
        limit: Option<u32>,
        unread_only: bool,
    ) -> Result<Vec<Notification>, NotificationError>;

    async fn get_notification(
        &self,
        notification_id: NotificationId,
    ) -> Result<Notification, NotificationError>;

    async fn mark_as_read(
        &self,
        notification_id: NotificationId,
        actor_id: &str,
    ) -> Result<(), NotificationError>;

    async fn publish_event(
        &self,
        payload: NotificationEventPayload,
    ) -> Result<(), NotificationError>;
}
