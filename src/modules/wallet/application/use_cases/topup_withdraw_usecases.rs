use rust_decimal::Decimal;
use uuid::Uuid;

use crate::modules::wallet::domain::entities::TransactionType;
use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::application::dto::{TopupRequest, TopupResponse, WithdrawRequest, WithdrawResponse};
use super::WalletUseCases;

impl WalletUseCases {
    pub async fn topup(&self, user_id: Uuid, req: TopupRequest) -> Result<TopupResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let amount_decimal = Self::from_cents(req.amount_cents);

        // Ensure wallet exists
        if self.repo.get_wallet(user_id).await?.is_none() {
            self.repo.create_wallet(user_id).await?;
        }

        // Add balance
        let updated_wallet = self
            .repo
            .update_balances(user_id, amount_decimal, Decimal::ZERO)
            .await?;

        // Record transaction
        let tx = self
            .repo
            .create_transaction(user_id, TransactionType::TOPUP, amount_decimal, None)
            .await?;

        Ok(TopupResponse {
            topup_id: tx.id,
            status: "COMPLETED".to_string(),
            new_available_cents: Self::to_cents(updated_wallet.balance),
        })
    }

    pub async fn withdraw(&self, user_id: Uuid, req: WithdrawRequest) -> Result<WithdrawResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let amount_decimal = Self::from_cents(req.amount_cents);

        let wallet = self.repo.get_wallet(user_id).await?.ok_or(WalletError::NotFound)?;
        if wallet.balance < amount_decimal {
            return Err(WalletError::InsufficientBalance);
        }

        // Deduct available balance
        self.repo
            .update_balances(user_id, -amount_decimal, Decimal::ZERO)
            .await?;

        // Record transaction
        let tx = self
            .repo
            .create_transaction(user_id, TransactionType::WITHDRAW, amount_decimal, None)
            .await?;

        Ok(WithdrawResponse {
            withdraw_id: tx.id,
            status: "PENDING_REVIEW".to_string(),
        })
    }
}
