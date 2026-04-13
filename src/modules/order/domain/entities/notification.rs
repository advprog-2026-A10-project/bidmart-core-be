use super::order::OrderId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: NotificationId,
    pub order_id: Option<OrderId>,
    pub title: String,
    pub body: String,
    pub channel: NotificationChannel,
    pub notification_type: NotificationType,
    pub created_at: String,
    pub read_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub type NotificationId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    Push,
    Inbox,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationType {
    BidPlaced,
    WinnerDetermined,
    OrderUpdate,
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationEventPayload {
    pub order_id: Option<OrderId>,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub channel: NotificationChannel,
    pub metadata: Option<serde_json::Value>,
}
