use uuid::Uuid;

use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::application::dto::WalletBalanceResponse;
use super::WalletUseCases;

impl WalletUseCases {
    pub async fn get_balance(&self, user_id: Uuid) -> Result<WalletBalanceResponse, WalletError> {
        let wallet = match self.repo.get_wallet(user_id).await? {
            Some(w) => w,
            None => self.repo.create_wallet(user_id).await?,
        };

        Ok(WalletBalanceResponse {
            available_cents: Self::to_cents(wallet.balance),
            held_cents: Self::to_cents(wallet.held_balance),
            currency: "IDR".to_string(),
        })
    }
}
