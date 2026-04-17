use rust_decimal::Decimal;

use crate::modules::wallet::domain::entities::TransactionType;
use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::application::dto::{
    InternalHoldRequest, InternalHoldResponse, InternalPaymentRequest, InternalPaymentResponse,
    InternalReleaseRequest, InternalReleaseResponse,
};
use super::WalletUseCases;

impl WalletUseCases {
    pub async fn internal_hold(&self, req: InternalHoldRequest) -> Result<InternalHoldResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let amount_decimal = Self::from_cents(req.amount_cents);

        // Ensure wallet exists
        let wallet = match self.repo.get_wallet(req.user_id).await? {
            Some(w) => w,
            None => return Err(WalletError::NotFound),
        };

        if wallet.balance < amount_decimal {
            return Err(WalletError::InsufficientBalance);
        }

        // Move to held balance (balance -= amount, held += amount)
        let updated_wallet = self
            .repo
            .update_balances(req.user_id, -amount_decimal, amount_decimal)
            .await?;

        // Record hold transaction
        let tx = self
            .repo
            .create_transaction(req.user_id, TransactionType::HOLD, amount_decimal, Some(req.bid_id))
            .await?;

        Ok(InternalHoldResponse {
            held: true,
            hold_id: tx.id,
            available_cents: Self::to_cents(updated_wallet.balance),
            held_cents: Self::to_cents(updated_wallet.held_balance),
        })
    }

    pub async fn internal_release(
        &self,
        req: InternalReleaseRequest,
    ) -> Result<InternalReleaseResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let amount_decimal = Self::from_cents(req.amount_cents);

        let wallet = match self.repo.get_wallet(req.user_id).await? {
            Some(w) => w,
            None => return Err(WalletError::NotFound),
        };

        if wallet.held_balance < amount_decimal {
            return Err(WalletError::InsufficientBalance);
        }

        // Move from held balance back to available balance (balance += amount, held -= amount)
        let updated_wallet = self
            .repo
            .update_balances(req.user_id, amount_decimal, -amount_decimal)
            .await?;

        // Record release transaction
        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::RELEASE,
                amount_decimal,
                Some(req.reference_id),
            )
            .await?;

        Ok(InternalReleaseResponse {
            released: true,
            release_id: tx.id,
            available_cents: Self::to_cents(updated_wallet.balance),
            held_cents: Self::to_cents(updated_wallet.held_balance),
        })
    }

    pub async fn internal_payment(
        &self,
        req: InternalPaymentRequest,
    ) -> Result<InternalPaymentResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::InvalidAmount);
        }

        let amount_decimal = Self::from_cents(req.amount_cents);

        let wallet = match self.repo.get_wallet(req.user_id).await? {
            Some(w) => w,
            None => return Err(WalletError::NotFound),
        };

        if wallet.held_balance < amount_decimal {
            return Err(WalletError::InsufficientBalance);
        }

        // Permanently deduct from held balance (balance += 0, held -= amount)
        let updated_wallet = self
            .repo
            .update_balances(req.user_id, Decimal::ZERO, -amount_decimal)
            .await?;

        // Record payment transaction
        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::PAYMENT,
                amount_decimal,
                Some(req.reference_id),
            )
            .await?;

        Ok(InternalPaymentResponse {
            paid: true,
            payment_id: tx.id,
            available_cents: Self::to_cents(updated_wallet.balance),
            held_cents: Self::to_cents(updated_wallet.held_balance),
        })
    }
}
