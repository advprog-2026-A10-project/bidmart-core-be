use std::sync::Arc;
use rust_decimal::Decimal;
use crate::modules::wallet::domain::traits::WalletRepository;

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

    fn to_cents(amount: Decimal) -> i64 {
        (amount * Decimal::new(100, 0)).to_string().parse::<i64>().unwrap_or(0)
    }

    fn from_cents(amount_cents: i64) -> Decimal {
        Decimal::new(amount_cents, 2)
    }
}
