use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: OrderId,
    pub lot: String,
    pub stage: OrderStage,
    pub status: OrderStatus,
    pub buyer_id: String,
    pub seller_id: String,
    pub total: String,
    pub currency: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: Vec<String>,
    pub last_activity: String,
}

pub type OrderId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "order_stage", rename_all = "lowercase")]
pub enum OrderStage {
    Active,
    Processing,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "order_status", rename_all = "snake_case")]
pub enum OrderStatus {
    AwaitingPayment,
    InTransit,
    NeedsConfirmation,
    Delivered,
    DisputeClosed,
    DisputeAlert,
}
