use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
