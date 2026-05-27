use sqlx::{Postgres, Row, Transaction};
use thiserror::Error;
use uuid::Uuid;

use crate::modules::wallet::domain::entities::{ReferenceType, TransactionStatus, TransactionType};

#[derive(Debug, Error)]
pub enum WalletLedgerError {
    #[error("insufficient available balance")]
    InsufficientBalance,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

async fn ensure_wallet_exists(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<(), WalletLedgerError> {
    sqlx::query(
        r#"
        INSERT INTO wallets (user_id, balance, held_balance)
        VALUES ($1, 0, 0)
        ON CONFLICT (user_id) DO NOTHING
        "#,
    )
    .bind(user_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_completed_transaction(
    tx: &mut Transaction<'_, Postgres>,
    wallet_id: Uuid,
    tx_type: TransactionType,
    amount: i64,
    balance_after: i64,
    reference_id: Uuid,
    reference_type: ReferenceType,
    description: String,
) -> Result<(), WalletLedgerError> {
    sqlx::query(
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
        VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8,
            CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(wallet_id)
    .bind(tx_type)
    .bind(TransactionStatus::Completed)
    .bind(amount)
    .bind(balance_after)
    .bind(reference_id)
    .bind(reference_type)
    .bind(description)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn hold_bid_funds(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    auction_id: Uuid,
    amount_to_hold: i64,
) -> Result<(), WalletLedgerError> {
    if amount_to_hold <= 0 {
        return Ok(());
    }

    let wallet_row = sqlx::query(
        r#"
        SELECT balance, held_balance
        FROM wallets
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(WalletLedgerError::InsufficientBalance)?;

    let balance: i64 = wallet_row.try_get("balance")?;
    let held_balance: i64 = wallet_row.try_get("held_balance")?;
    let available = balance.saturating_sub(held_balance);
    if available < amount_to_hold {
        return Err(WalletLedgerError::InsufficientBalance);
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET held_balance = held_balance + $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND (held_balance + $2) <= balance
        RETURNING balance
        "#,
    )
    .bind(user_id)
    .bind(amount_to_hold)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(WalletLedgerError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    insert_completed_transaction(
        tx,
        user_id,
        TransactionType::BidHold,
        amount_to_hold,
        balance_after,
        auction_id,
        ReferenceType::Auction,
        format!("Bid hold for auction {}", auction_id),
    )
    .await
}

pub async fn release_bid_hold(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    auction_id: Uuid,
    amount_to_release: i64,
) -> Result<(), WalletLedgerError> {
    if amount_to_release <= 0 {
        return Ok(());
    }

    let wallet_row = sqlx::query(
        r#"
        SELECT held_balance
        FROM wallets
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(WalletLedgerError::InsufficientBalance)?;

    let held_balance: i64 = wallet_row.try_get("held_balance")?;
    if held_balance < amount_to_release {
        return Err(WalletLedgerError::InsufficientBalance);
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET held_balance = held_balance - $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND (held_balance - $2) >= 0
        RETURNING balance
        "#,
    )
    .bind(user_id)
    .bind(amount_to_release)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(WalletLedgerError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    insert_completed_transaction(
        tx,
        user_id,
        TransactionType::BidRelease,
        amount_to_release,
        balance_after,
        auction_id,
        ReferenceType::Auction,
        format!("Bid release for auction {}", auction_id),
    )
    .await
}

pub async fn convert_hold_to_payment(
    tx: &mut Transaction<'_, Postgres>,
    winner_id: Uuid,
    auction_id: Uuid,
    amount: i64,
) -> Result<(), WalletLedgerError> {
    if amount <= 0 {
        return Err(WalletLedgerError::Validation(
            "winning amount must be greater than 0".to_string(),
        ));
    }

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET balance = balance - $2,
            held_balance = held_balance - $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
          AND held_balance >= $2
          AND balance >= $2
        RETURNING balance
        "#,
    )
    .bind(winner_id)
    .bind(amount)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(WalletLedgerError::InsufficientBalance)?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    insert_completed_transaction(
        tx,
        winner_id,
        TransactionType::BidConvert,
        amount,
        balance_after,
        auction_id,
        ReferenceType::Auction,
        format!("Bid convert for auction {}", auction_id),
    )
    .await
}

pub async fn credit_auction_payment(
    tx: &mut Transaction<'_, Postgres>,
    seller_id: Uuid,
    auction_id: Uuid,
    amount: i64,
) -> Result<(), WalletLedgerError> {
    if amount <= 0 {
        return Err(WalletLedgerError::Validation(
            "payment amount must be greater than 0".to_string(),
        ));
    }

    ensure_wallet_exists(tx, seller_id).await?;

    let updated_wallet = sqlx::query(
        r#"
        UPDATE wallets
        SET balance = balance + $2,
            updated_at = CURRENT_TIMESTAMP
        WHERE user_id = $1
        RETURNING balance
        "#,
    )
    .bind(seller_id)
    .bind(amount)
    .fetch_one(&mut **tx)
    .await?;

    let balance_after: i64 = updated_wallet.try_get("balance")?;

    insert_completed_transaction(
        tx,
        seller_id,
        TransactionType::PaymentReceived,
        amount,
        balance_after,
        auction_id,
        ReferenceType::Auction,
        format!("Payment received from auction {}", auction_id),
    )
    .await
}
