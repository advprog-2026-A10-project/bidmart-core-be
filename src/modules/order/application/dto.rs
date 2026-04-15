use crate::modules::order::domain::entities::{NotificationEventPayload, OrderId, OrderStage};
use uuid::Uuid;

pub struct ListOrdersDto {
    pub user_id: Option<String>,
    pub role: String,
    pub stage: Option<OrderStage>,
}

pub struct GetOrderDto {
    pub order_id: OrderId,
}

pub struct ConfirmOrderDto {
    pub order_id: OrderId,
    pub actor_id: String,
}

pub struct CreateDisputeDto {
    pub order_id: OrderId,
    pub reporter_id: String,
    pub reason: String,
}

pub struct UpdateShippingDto {
    pub order_id: OrderId,
    pub status: String,
    pub tracking: Option<String>,
}

pub struct ListNotificationsDto {
    pub user_id: Option<String>,
    pub limit: Option<u32>,
    pub unread_only: bool,
}

pub struct GetNotificationDto {
    pub notification_id: Uuid,
}

pub struct MarkNotificationDto {
    pub notification_id: Uuid,
    pub actor_id: String,
}

pub struct PublishEventDto {
    pub payload: NotificationEventPayload,
}
