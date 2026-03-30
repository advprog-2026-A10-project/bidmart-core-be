use std::sync::Arc;

use rust_decimal::Decimal;
use uuid::Uuid;

use crate::modules::wallet::domain::entities::TransactionType;
use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::domain::traits::WalletRepository;

use super::dto::{
    InternalHoldRequest, InternalHoldResponse, ReferenceDto, TopupRequest, TopupResponse,
    TransactionDto, TransactionListResponse, WalletBalanceResponse, WithdrawRequest,
    WithdrawResponse,
};

pub struct WalletUseCases {
    repo: Arc<dyn WalletRepository>,
}

impl WalletUseCases {
    pub fn new(repo: Arc<dyn WalletRepository>) -> Self {
        Self { repo }
    }

    fn to_cents(amount: Decimal) -> i64 {
        // Assuming database stores decimal
        (amount * Decimal::new(100, 0)).to_string().parse::<i64>().unwrap_or(0)
    }

    fn from_cents(amount_cents: i64) -> Decimal {
        Decimal::new(amount_cents, 2)
    }

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

    pub async fn list_transactions(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<TransactionListResponse, WalletError> {
        let (transactions, total) = self.repo.list_transactions(user_id, page, page_size).await?;

        let data = transactions
            .into_iter()
            .map(|tx| TransactionDto {
                tx_id: tx.id,
                r#type: format!("{:?}", tx.r#type),
                amount_cents: Self::to_cents(tx.amount),
                created_at: tx.created_at,
                ref_info: tx.reference_id.map(|id| ReferenceDto {
                    r#type: "BID".to_string(), // hardcoded for example, should be dynamic if possible
                    id,
                }),
            })
            .collect();

        Ok(TransactionListResponse {
            data,
            page,
            page_size,
            total,
        })
    }

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

    pub async fn get_transaction_by_id(
        &self,
        user_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<TransactionDto, WalletError> {
        let tx = self
            .repo
            .get_transaction_by_id(user_id, transaction_id)
            .await?
            .ok_or(WalletError::NotFound)?;

        Ok(TransactionDto {
            tx_id: tx.id,
            r#type: format!("{:?}", tx.r#type),
            amount_cents: Self::to_cents(tx.amount),
            created_at: tx.created_at,
            ref_info: tx.reference_id.map(|id| ReferenceDto {
                r#type: "BID".to_string(),
                id,
            }),
        })
    }
}
