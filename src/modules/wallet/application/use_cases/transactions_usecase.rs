use uuid::Uuid;

use super::WalletUseCases;
use crate::modules::wallet::application::dto::{
    ReferenceDto, TransactionDto, TransactionListResponse,
};
use crate::modules::wallet::domain::entities::{ReferenceType, TransactionStatus, TransactionType};
use crate::modules::wallet::domain::errors::WalletError;

impl WalletUseCases {
    fn to_tx_type_label(tx_type: &TransactionType) -> &'static str {
        match tx_type {
            TransactionType::Topup => "TOPUP",
            TransactionType::Withdraw => "WITHDRAW",
            TransactionType::BidHold => "BID_HOLD",
            TransactionType::BidRelease => "BID_RELEASE",
            TransactionType::BidConvert => "BID_CONVERT",
            TransactionType::PaymentReceived => "PAYMENT_RECEIVED",
            TransactionType::Refund => "REFUND",
        }
    }

    fn to_tx_status_label(status: &TransactionStatus) -> &'static str {
        match status {
            TransactionStatus::Pending => "PENDING",
            TransactionStatus::Completed => "COMPLETED",
            TransactionStatus::Failed => "FAILED",
            TransactionStatus::Cancelled => "CANCELLED",
        }
    }

    fn to_reference_label(reference_type: &ReferenceType) -> &'static str {
        match reference_type {
            ReferenceType::Auction => "AUCTION",
            ReferenceType::Order => "ORDER",
            ReferenceType::Dispute => "DISPUTE",
            ReferenceType::Topup => "TOPUP",
            ReferenceType::Withdraw => "WITHDRAW",
        }
    }

    pub async fn list_transactions(
        &self,
        user_id: Uuid,
        page: i64,
        page_size: i64,
    ) -> Result<TransactionListResponse, WalletError> {
        if page <= 0 || page_size <= 0 {
            return Err(WalletError::ValidationError(
                "page and pageSize must be greater than 0".to_string(),
            ));
        }

        let (transactions, total) = self
            .repo
            .list_transactions(user_id, page, page_size)
            .await?;

        let data = transactions
            .into_iter()
            .map(|tx| TransactionDto {
                tx_id: tx.id,
                r#type: Self::to_tx_type_label(&tx.r#type).to_string(),
                status: Self::to_tx_status_label(&tx.status).to_string(),
                amount_cents: tx.amount,
                balance_after_cents: tx.balance_after,
                created_at: tx.created_at,
                ref_info: tx.reference_id.and_then(|id| {
                    tx.reference_type
                        .as_ref()
                        .map(|reference_type| ReferenceDto {
                            r#type: Self::to_reference_label(reference_type).to_string(),
                            id,
                        })
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
            r#type: Self::to_tx_type_label(&tx.r#type).to_string(),
            status: Self::to_tx_status_label(&tx.status).to_string(),
            amount_cents: tx.amount,
            balance_after_cents: tx.balance_after,
            created_at: tx.created_at,
            ref_info: tx.reference_id.and_then(|id| {
                tx.reference_type
                    .as_ref()
                    .map(|reference_type| ReferenceDto {
                        r#type: Self::to_reference_label(reference_type).to_string(),
                        id,
                    })
            }),
        })
    }
}
