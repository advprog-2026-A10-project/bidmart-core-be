use std::sync::Arc;

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::wallet::domain::entities::{TransactionType, Wallet, WalletTransaction};
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
        balance_delta: Decimal,
        held_delta: Decimal,
    ) -> Result<Wallet, WalletError> {
        let wallet = sqlx::query_as::<_, Wallet>(
            r#"
            UPDATE wallets
            SET balance = balance + $2,
                held_balance = held_balance + $3,
                updated_at = CURRENT_TIMESTAMP
            WHERE user_id = $1
            RETURNING user_id, balance, held_balance, updated_at
            "#,
        )
        .bind(user_id)
        .bind(balance_delta)
        .bind(held_delta)
        .fetch_one(&*self.pool)
        .await?;

        Ok(wallet)
    }

    async fn create_transaction(
        &self,
        wallet_id: Uuid,
        tx_type: TransactionType,
        amount: Decimal,
        reference_id: Option<Uuid>,
    ) -> Result<WalletTransaction, WalletError> {
        let transaction = sqlx::query_as::<_, WalletTransaction>(
            r#"
            INSERT INTO wallet_transactions (wallet_id, type, amount, reference_id)
            VALUES ($1, $2, $3, $4)
            RETURNING id, wallet_id, type, amount, reference_id, created_at
            "#,
        )
        .bind(wallet_id)
        .bind(tx_type)
        .bind(amount)
        .bind(reference_id)
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
            SELECT id, wallet_id, type, amount, reference_id, created_at
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
            SELECT id, wallet_id, type, amount, reference_id, created_at
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
