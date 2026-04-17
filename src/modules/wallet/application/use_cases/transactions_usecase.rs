use uuid::Uuid;

use crate::modules::wallet::domain::errors::WalletError;
use crate::modules::wallet::application::dto::{ReferenceDto, TransactionDto, TransactionListResponse};
use super::WalletUseCases;

impl WalletUseCases {
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
