use uuid::Uuid;

use super::WalletUseCases;
use crate::modules::wallet::application::dto::{
    TopupRequest, TopupResponse, WithdrawRequest, WithdrawResponse,
};
use crate::modules::wallet::domain::entities::{TransactionStatus, TransactionType};
use crate::modules::wallet::domain::errors::WalletError;

impl WalletUseCases {
    pub async fn topup(
        &self,
        user_id: Uuid,
        req: TopupRequest,
    ) -> Result<TopupResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }
        if req.method.trim().is_empty() {
            return Err(WalletError::ValidationError(
                "method cannot be empty".to_string(),
            ));
        }

        if self.repo.get_wallet(user_id).await?.is_none() {
            self.repo.create_wallet(user_id).await?;
        }

        let updated_wallet = self
            .repo
            .update_balances(user_id, req.amount_cents, 0)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                user_id,
                TransactionType::Topup,
                TransactionStatus::Completed,
                req.amount_cents,
                updated_wallet.balance,
                None,
                None,
                format!("Top up via {}", req.method.trim()),
            )
            .await?;

        Ok(TopupResponse {
            topup_id: tx.id,
            status: "COMPLETED".to_string(),
            new_available_cents: Self::available_cents(
                updated_wallet.balance,
                updated_wallet.held_balance,
            ),
        })
    }

    pub async fn withdraw(
        &self,
        user_id: Uuid,
        req: WithdrawRequest,
    ) -> Result<WithdrawResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }
        if req.bank_account.bank.trim().is_empty()
            || req.bank_account.account_no.trim().is_empty()
            || req.bank_account.name.trim().is_empty()
        {
            return Err(WalletError::ValidationError(
                "bankAccount fields cannot be empty".to_string(),
            ));
        }

        let wallet = self
            .repo
            .get_wallet(user_id)
            .await?
            .ok_or(WalletError::NotFound)?;
        let available = Self::available_cents(wallet.balance, wallet.held_balance);
        if available < req.amount_cents {
            return Err(WalletError::InsufficientBalance);
        }

        let updated_wallet = self
            .repo
            .update_balances(user_id, -req.amount_cents, 0)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                user_id,
                TransactionType::Withdraw,
                TransactionStatus::Pending,
                req.amount_cents,
                updated_wallet.balance,
                None,
                None,
                format!(
                    "Withdraw to {} {} ({})",
                    req.bank_account.bank.trim(),
                    req.bank_account.account_no.trim(),
                    req.bank_account.name.trim()
                ),
            )
            .await?;

        Ok(WithdrawResponse {
            withdraw_id: tx.id,
            status: "PENDING_REVIEW".to_string(),
        })
    }
}
