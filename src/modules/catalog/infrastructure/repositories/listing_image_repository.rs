use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingImage;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::ListingImageRepository;

pub struct PostgresListingImageRepository {
    pool: PgPool,
}

impl PostgresListingImageRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ListingImageRepository for PostgresListingImageRepository {
    async fn get_listing_images(
        &self,
        listing_id: Uuid,
    ) -> Result<Vec<ListingImage>, ListingError> {
        sqlx::query_as::<_, ListingImage>(
            r#"
            SELECT id, listing_id, url, "order", created_at
            FROM listing_images
            WHERE listing_id = $1
            ORDER BY "order" ASC
            "#,
        )
        .bind(listing_id)
        .fetch_all(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)
    }

    async fn replace_listing_images(
        &self,
        listing_id: Uuid,
        urls: Vec<String>,
    ) -> Result<Vec<ListingImage>, ListingError> {
        let mut tx = self.pool.begin().await.map_err(ListingError::DatabaseError)?;

        // Delete existing images
        sqlx::query("DELETE FROM listing_images WHERE listing_id = $1")
            .bind(listing_id)
            .execute(&mut *tx)
            .await
            .map_err(ListingError::DatabaseError)?;

        // Insert new images — index position becomes the order value
        for (order, url) in urls.into_iter().enumerate() {
            sqlx::query(
                r#"INSERT INTO listing_images (listing_id, url, "order") VALUES ($1, $2, $3)"#,
            )
            .bind(listing_id)
            .bind(url)
            .bind(order as i32)
            .execute(&mut *tx)
            .await
            .map_err(ListingError::DatabaseError)?;
        }

        // Fetch and return the newly inserted images
        let images = sqlx::query_as::<_, ListingImage>(
            r#"
            SELECT id, listing_id, url, "order", created_at
            FROM listing_images
            WHERE listing_id = $1
            ORDER BY "order" ASC
            "#,
        )
        .bind(listing_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(ListingError::DatabaseError)?;

        tx.commit().await.map_err(ListingError::DatabaseError)?;
        Ok(images)
    }
}
