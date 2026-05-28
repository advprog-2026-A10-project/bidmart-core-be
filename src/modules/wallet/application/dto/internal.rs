use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct InternalHoldRequest {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    /// Auction the hold belongs to. Surfaces in `wallet_transactions.reference_id`
    /// with type `Auction` so the audit trail (WBS 4.2.4) can link the hold
    /// back to a navigable entity.
    #[serde(rename = "auctionId")]
    pub auction_id: Uuid,
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

/// Credit a seller's wallet for an auction payout (counterpart of
/// `internal_payment`, which debits the buyer's held funds). The two together
/// fulfil WBS 4.2.3 — convert hold into payment to seller.
#[derive(Deserialize)]
pub struct InternalCreditRequest {
    #[serde(rename = "userId")]
    pub user_id: Uuid,
    #[serde(rename = "referenceId")]
    pub reference_id: Uuid,
    #[serde(rename = "amountCents")]
    pub amount_cents: i64,
}

#[derive(Serialize)]
pub struct InternalCreditResponse {
    pub credited: bool,
    #[serde(rename = "creditId")]
    pub credit_id: Uuid,
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
}
