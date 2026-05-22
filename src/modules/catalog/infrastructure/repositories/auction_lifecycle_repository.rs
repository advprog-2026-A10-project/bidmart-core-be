use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::Listing;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::AuctionLifecyclePort;

/// Concrete adapter that inserts the `auctions` row and links the listing in
/// a single SQL transaction so the catalog publish flow stays consistent.
pub struct PostgresAuctionLifecycleRepository {
    pool: PgPool,
}

impl PostgresAuctionLifecycleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AuctionLifecyclePort for PostgresAuctionLifecycleRepository {
    async fn start_auction_for_listing(
        &self,
        listing: &Listing,
        image_url: Option<String>,
    ) -> Result<Uuid, ListingError> {
        let mut tx = self.pool.begin().await.map_err(ListingError::DatabaseError)?;

        let now = Utc::now();
        let initial_status = if listing.starts_at <= now {
            "ACTIVE"
        } else {
            "SCHEDULED"
        };
        let image = image_url.unwrap_or_default();

        let row = sqlx::query(
            r#"
            INSERT INTO auctions (
                listing_id,
                seller_id,
                seller_name,
                title,
                description,
                image_url,
                start_price,
                current_price,
                reserve_price,
                bid_increment,
                bid_count,
                status,
                starts_at,
                ends_at,
                original_ends_at,
                extension_count
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $7, $8, $9, 0,
                $10::auction_status, $11, $12, $12, 0
            )
            RETURNING id
            "#,
        )
        .bind(listing.id)
        .bind(listing.seller_id)
        .bind(&listing.seller_name)
        .bind(&listing.title)
        .bind(&listing.description)
        .bind(&image)
        .bind(listing.start_price)
        .bind(listing.reserve_price)
        .bind(listing.min_increment)
        .bind(initial_status)
        .bind(listing.starts_at)
        .bind(listing.ends_at)
        .fetch_one(&mut *tx)
        .await
        .map_err(ListingError::DatabaseError)?;

        let auction_id: Uuid = row.try_get("id").map_err(ListingError::DatabaseError)?;

        let affected = sqlx::query(
            "UPDATE listings SET auction_id = $2, updated_at = NOW() WHERE id = $1",
        )
        .bind(listing.id)
        .bind(auction_id)
        .execute(&mut *tx)
        .await
        .map_err(ListingError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(ListingError::DatabaseError)?;
            return Err(ListingError::NotFound);
        }

        tx.commit().await.map_err(ListingError::DatabaseError)?;
        Ok(auction_id)
    }
}
