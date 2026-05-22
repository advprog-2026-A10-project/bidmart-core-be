use uuid::Uuid;

use super::WalletUseCases;
use crate::modules::wallet::application::dto::WalletBalanceResponse;
use crate::modules::wallet::domain::errors::WalletError;

impl WalletUseCases {
    pub async fn get_balance(&self, user_id: Uuid) -> Result<WalletBalanceResponse, WalletError> {
        let wallet = match self.repo.get_wallet(user_id).await? {
            Some(w) => w,
            None => self.repo.create_wallet(user_id).await?,
        };

        Ok(WalletBalanceResponse {
            available_cents: Self::available_cents(wallet.balance, wallet.held_balance),
            held_cents: wallet.held_balance,
            currency: "IDR".to_string(),
        })
    }
}
