use crate::modules::order::domain::entities::{OrderId, OrderStage};

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
