use super::WalletUseCases;
use crate::modules::wallet::application::dto::{
    InternalCreditRequest, InternalCreditResponse, InternalHoldRequest, InternalHoldResponse,
    InternalPaymentRequest, InternalPaymentResponse, InternalReleaseRequest,
    InternalReleaseResponse,
};
use crate::modules::wallet::domain::entities::{ReferenceType, TransactionStatus, TransactionType};
use crate::modules::wallet::domain::errors::WalletError;

impl WalletUseCases {
    pub async fn internal_hold(
        &self,
        req: InternalHoldRequest,
    ) -> Result<InternalHoldResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }
        let wallet = self
            .repo
            .get_wallet(req.user_id)
            .await?
            .ok_or(WalletError::NotFound)?;
        let available = Self::available_cents(wallet.balance, wallet.held_balance);
        if available < req.amount_cents {
            return Err(WalletError::InsufficientBalance);
        }

        // hold funds while preserving total balance.
        let updated_wallet = self
            .repo
            .update_balances(req.user_id, 0, req.amount_cents)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::BidHold,
                TransactionStatus::Completed,
                req.amount_cents,
                updated_wallet.balance,
                Some(req.auction_id),
                Some(ReferenceType::Auction),
                format!("Bid hold listing={} bid={}", req.listing_id, req.bid_id),
            )
            .await?;

        Ok(InternalHoldResponse {
            held: true,
            hold_id: tx.id,
            available_cents: Self::available_cents(
                updated_wallet.balance,
                updated_wallet.held_balance,
            ),
            held_cents: updated_wallet.held_balance,
        })
    }

    pub async fn internal_release(
        &self,
        req: InternalReleaseRequest,
    ) -> Result<InternalReleaseResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }

        let wallet = self
            .repo
            .get_wallet(req.user_id)
            .await?
            .ok_or(WalletError::NotFound)?;

        if wallet.held_balance < req.amount_cents {
            return Err(WalletError::InsufficientBalance);
        }

        let updated_wallet = self
            .repo
            .update_balances(req.user_id, 0, -req.amount_cents)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::BidRelease,
                TransactionStatus::Completed,
                req.amount_cents,
                updated_wallet.balance,
                Some(req.reference_id),
                Some(ReferenceType::Auction),
                format!("Bid release reference={}", req.reference_id),
            )
            .await?;

        Ok(InternalReleaseResponse {
            released: true,
            release_id: tx.id,
            available_cents: Self::available_cents(
                updated_wallet.balance,
                updated_wallet.held_balance,
            ),
            held_cents: updated_wallet.held_balance,
        })
    }

    pub async fn internal_payment(
        &self,
        req: InternalPaymentRequest,
    ) -> Result<InternalPaymentResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }

        let wallet = self
            .repo
            .get_wallet(req.user_id)
            .await?
            .ok_or(WalletError::NotFound)?;

        if wallet.held_balance < req.amount_cents {
            return Err(WalletError::InsufficientBalance);
        }

        // convert held funds into final payment.
        let updated_wallet = self
            .repo
            .update_balances(req.user_id, -req.amount_cents, -req.amount_cents)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::BidConvert,
                TransactionStatus::Completed,
                req.amount_cents,
                updated_wallet.balance,
                Some(req.reference_id),
                Some(ReferenceType::Auction),
                format!("Bid convert reference={}", req.reference_id),
            )
            .await?;

        Ok(InternalPaymentResponse {
            paid: true,
            payment_id: tx.id,
            available_cents: Self::available_cents(
                updated_wallet.balance,
                updated_wallet.held_balance,
            ),
            held_cents: updated_wallet.held_balance,
        })
    }

    /// Credit a seller's wallet for an auction payout. Counterpart of
    /// `internal_payment`: `internal_payment` debits the buyer's held funds,
    /// `internal_credit` adds the same amount to the seller's available
    /// balance. Together they fulfil WBS 4.2.3 (convert hold → payment) end
    /// to end without bidding needing direct table access.
    pub async fn internal_credit(
        &self,
        req: InternalCreditRequest,
    ) -> Result<InternalCreditResponse, WalletError> {
        if req.amount_cents <= 0 {
            return Err(WalletError::ValidationError(
                "amountCents must be greater than 0".to_string(),
            ));
        }

        // Lazily create the seller wallet so the first auction win does not
        // require a separate provisioning step.
        if self.repo.get_wallet(req.user_id).await?.is_none() {
            self.repo.create_wallet(req.user_id).await?;
        }

        let updated_wallet = self
            .repo
            .update_balances(req.user_id, req.amount_cents, 0)
            .await?;

        let tx = self
            .repo
            .create_transaction(
                req.user_id,
                TransactionType::PaymentReceived,
                TransactionStatus::Completed,
                req.amount_cents,
                updated_wallet.balance,
                Some(req.reference_id),
                Some(ReferenceType::Auction),
                format!("Auction payment received reference={}", req.reference_id),
            )
            .await?;

        Ok(InternalCreditResponse {
            credited: true,
            credit_id: tx.id,
            available_cents: Self::available_cents(
                updated_wallet.balance,
                updated_wallet.held_balance,
            ),
            held_cents: updated_wallet.held_balance,
        })
    }
}
