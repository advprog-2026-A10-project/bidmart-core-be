use sqlx::PgPool;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::domain::errors::ListingError;

pub struct PostgresListingIntegrationRepository {
    pool: PgPool,
}

impl PostgresListingIntegrationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_listing_status(
        &self,
        id: Uuid,
    ) -> Result<Option<ListingStatus>, ListingError> {
        sqlx::query_scalar::<_, ListingStatus>(
            "SELECT status FROM listings WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)
    }

    pub async fn update_listing_status(
        &self,
        id: Uuid,
        status: ListingStatus,
    ) -> Result<(), ListingError> {
        let affected = sqlx::query(
            "UPDATE listings SET status = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(ListingError::NotFound);
        }
        Ok(())
    }

    pub async fn link_auction(
        &self,
        listing_id: Uuid,
        auction_id: Uuid,
    ) -> Result<(), ListingError> {
        let affected = sqlx::query(
            "UPDATE listings SET auction_id = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(listing_id)
        .bind(auction_id)
        .execute(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(ListingError::NotFound);
        }
        Ok(())
    }

    pub async fn update_ends_at(
        &self,
        listing_id: Uuid,
        new_ends_at: DateTime<Utc>,
    ) -> Result<(), ListingError> {
        let affected = sqlx::query(
            "UPDATE listings SET ends_at = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(listing_id)
        .bind(new_ends_at)
        .execute(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(ListingError::NotFound);
        }
        Ok(())
    }
}
