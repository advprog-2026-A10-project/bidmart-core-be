use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
