use std::sync::Arc;

use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::wallet::domain::entities::{
    ReferenceType, TransactionStatus, TransactionType, Wallet, WalletTransaction,
};
use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::domain::traits::WalletRepository;

pub struct PostgresWalletRepository {
    pool: Arc<PgPool>,
}

impl PostgresWalletRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl WalletRepository for PostgresWalletRepository {
    async fn get_wallet(&self, user_id: Uuid) -> Result<Option<Wallet>, WalletError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            SELECT user_id, balance, held_balance, updated_at
            FROM wallets
            WHERE user_id = $1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(wallet)
    }

    async fn create_wallet(&self, user_id: Uuid) -> Result<Wallet, WalletError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            INSERT INTO wallets (user_id, balance, held_balance)
            VALUES ($1, 0, 0)
            RETURNING user_id, balance, held_balance, updated_at
            "#,
        )
        .bind(user_id)
        .fetch_one(&*self.pool)
        .await?;

        Ok(wallet)
    }

    async fn update_balances(
        &self,
        user_id: Uuid,
        balance_delta: i64,
        held_delta: i64,
    ) -> Result<Wallet, WalletError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            UPDATE wallets
            SET balance = balance + $2,
                held_balance = held_balance + $3,
                updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $1
              AND (balance + $2) >= 0
              AND (held_balance + $3) >= 0
              AND (held_balance + $3) <= (balance + $2)
            RETURNING user_id, balance, held_balance, updated_at
            "#,
        )
        .bind(user_id)
        .bind(balance_delta)
        .bind(held_delta)
        .fetch_optional(&*self.pool)
        .await?;

        wallet.ok_or(WalletError::InsufficientBalance)
    }

    async fn create_transaction(
        &self,
        wallet_id: Uuid,
        tx_type: TransactionType,
        status: TransactionStatus,
        amount: i64,
        balance_after: i64,
        reference_id: Option<Uuid>,
        reference_type: Option<ReferenceType>,
        description: String,
    ) -> Result<WalletTransaction, WalletError> {
        let transaction = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions (
                wallet_id,
                type,
                status,
                amount,
                balance_after,
                reference_id,
                reference_type,
                description,
                completed_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CASE WHEN $3 = 'COMPLETED'::transaction_status THEN CURRENT_TIMESTAMP ELSE NULL END)
            RETURNING id, wallet_id, type, status, amount, balance_after, reference_id, reference_type, description, created_at, completed_at
            "#,
        )
        .bind(wallet_id)
        .bind(tx_type)
        .bind(status)
        .bind(amount)
        .bind(balance_after)
        .bind(reference_id)
        .bind(reference_type)
        .bind(description)
        .fetch_one(&*self.pool)
        .await?;

        Ok(transaction)
    }

    async fn list_transactions(
        &self,
        wallet_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<WalletTransaction>, i64), WalletError> {
        let offset = (page - 1) * page_size;

        let total_row: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM wallet_transactions WHERE wallet_id = $1
            "#,
        )
        .bind(wallet_id)
        .fetch_one(&*self.pool)
        .await?;

        let total = total_row.0;

        let transactions = sqlx::query_as::<_, WalletTransaction>(
            r#"
            SELECT id, wallet_id, type, status, amount, balance_after, reference_id, reference_type, description, created_at, completed_at
            FROM wallet_transactions
            WHERE wallet_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(wallet_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&*self.pool)
        .await?;

        Ok((transactions, total))
    }

    async fn get_transaction_by_id(
        &self,
        wallet_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Option<WalletTransaction>, WalletError> {
        let transaction = sqlx::query_as::<_, WalletTransaction>(
            r#"
            SELECT id, wallet_id, type, status, amount, balance_after, reference_id, reference_type, description, created_at, completed_at
            FROM wallet_transactions
            WHERE wallet_id = $1 AND id = $2
            "#,
        )
        .bind(wallet_id)
        .bind(transaction_id)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(transaction)
    }
}
