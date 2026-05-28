use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
pub struct TransactionListResponse {
    pub data: Vec<TransactionDto>,
    pub page: i64,
    #[serde(rename = "pageSize")]
    pub page_size: i64,
    pub total: i64,
}

#[derive(Serialize)]
pub struct TransactionDto {
    #[serde(rename = "txId")]
    pub tx_id: Uuid,
    pub r#type: String,
    pub status: String,
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
    #[serde(rename = "balanceAfterCents")]
    pub balance_after_cents: i64,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(rename = "refInfo")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_info: Option<ReferenceDto>,
}

#[derive(Serialize)]
pub struct ReferenceDto {
    pub r#type: String,
    pub id: Uuid,
}
