use crate::modules::wallet::domain::traits::WalletRepository;
use std::sync::Arc;

pub mod balance_usecase;
pub mod internal_usecases;
pub mod topup_withdraw_usecases;
pub mod transactions_usecase;

pub struct WalletUseCases {
    repo: Arc<dyn WalletRepository>,
}

impl WalletUseCases {
    pub fn new(repo: Arc<dyn WalletRepository>) -> Self {
        Self { repo }
    }

    fn available_cents(balance: i64, held_balance: i64) -> i64 {
        balance.saturating_sub(held_balance)
    }
}
