use serde::Serialize;

#[derive(Serialize)]
pub struct WalletBalanceResponse {
    #[serde(rename = "availableCents")]
    pub available_cents: i64,
    #[serde(rename = "heldCents")]
    pub held_cents: i64,
    pub currency: String,
}
