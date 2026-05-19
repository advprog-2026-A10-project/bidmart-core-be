use super::entities::{
    ReferenceType, TransactionStatus, TransactionType, Wallet, WalletTransaction,
};
use super::errors::WalletError;
use async_trait::async_trait;
use uuid::Uuid;

#[async_trait]
pub trait WalletRepository: Send + Sync {
    async fn get_wallet(&self, user_id: Uuid) -> Result<Option<Wallet>, WalletError>;

    async fn create_wallet(&self, user_id: Uuid) -> Result<Wallet, WalletError>;

    async fn update_balances(
        &self,
        user_id: Uuid,
        balance_delta: i64,
        held_delta: i64,
    ) -> Result<Wallet, WalletError>;

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
    ) -> Result<WalletTransaction, WalletError>;

    async fn list_transactions(
        &self,
        wallet_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<(Vec<WalletTransaction>, i64), WalletError>;

    async fn get_transaction_by_id(
        &self,
        wallet_id: Uuid,
        transaction_id: Uuid,
    ) -> Result<Option<WalletTransaction>, WalletError>;
}
