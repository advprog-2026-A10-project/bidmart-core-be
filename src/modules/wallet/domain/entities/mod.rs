use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use sqlx::FromRow;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Wallet {
    pub user_id: Uuid,
    pub balance: Decimal,
    pub held_balance: Decimal,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "transaction_type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TransactionType {
    TOPUP,
    WITHDRAW,
    HOLD,
    RELEASE,
    PAYMENT,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub r#type: TransactionType,
    pub amount: Decimal,
    pub reference_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
}
