use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct WalletBalanceResponse {
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
    pub currency: String,
}

#[derive(Deserialize)]
pub struct TopupRequest {
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
    pub method: String,
}

#[derive(Serialize)]
pub struct TopupResponse {
    #[serde(rename = "topupId")]
    pub topup_id: Uuid,
    pub status: String,
    #[serde(rename = "newAvailableCents")]
    pub new_available_cents: i64,
}

#[derive(Deserialize)]
pub struct WithdrawRequest {
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
    #[serde(rename = "bankAccount")]
    pub bank_account: BankAccountDto,
}

#[derive(Deserialize, Serialize)]
pub struct BankAccountDto {
    pub bank: String,
    #[serde(rename = "accountNo")]
    pub account_no: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct WithdrawResponse {
    #[serde(rename = "withdrawId")]
    pub withdraw_id: Uuid,
    pub status: String,
}

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
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
    #[serde(rename = "createdAt")]
    pub created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ref_info: Option<ReferenceDto>,
}

#[derive(Serialize)]
pub struct ReferenceDto {
    pub r#type: String,
    pub id: Uuid,
}

#[derive(Deserialize)]
pub struct InternalHoldRequest {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "listingId")]
    pub listing_id: Uuid,
    #[serde(rename = "bidId")]
    pub bid_id: Uuid,
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
}

#[derive(Serialize)]
pub struct InternalHoldResponse {
    pub held: bool,
    #[serde(rename = "holdId")]
    pub hold_id: Uuid,
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
}

#[derive(Deserialize)]
pub struct InternalReleaseRequest {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "referenceId")]
    pub reference_id: Uuid,
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
}

#[derive(Serialize)]
pub struct InternalReleaseResponse {
    pub released: bool,
    #[serde(rename = "releaseId")]
    pub release_id: Uuid,
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
}

#[derive(Deserialize)]
pub struct InternalPaymentRequest {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "referenceId")]
    pub reference_id: Uuid,
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
}

#[derive(Serialize)]
pub struct InternalPaymentResponse {
    pub paid: bool,
    #[serde(rename = "paymentId")]
    pub payment_id: Uuid,
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
}
