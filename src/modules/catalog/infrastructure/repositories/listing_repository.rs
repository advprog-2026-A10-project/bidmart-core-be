use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::Listing;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::domain::traits::{ListingFilter, ListingRepository};

pub struct PostgresListingRepository {
    pool: PgPool,
}

impl PostgresListingRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ListingRepository for PostgresListingRepository {
    async fn create_listing(
        &self,
        seller_id: Uuid,
        seller_name: String,
        category_id: Option<i32>,
        category_name: String,
        title: String,
        description: String,
        start_price: i64,
        reserve_price: Option<i64>,
        min_increment: i64,
        starts_at: DateTime<Utc>,
        ends_at: DateTime<Utc>,
    ) -> Result<Listing, ListingError> {
        sqlx::query_as::<_, Listing>(
            r#"
            INSERT INTO listings (
                seller_id, seller_name, category_id, category_name,
                title, description, start_price, reserve_price,
                current_price, min_increment, starts_at, ends_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING
                id, seller_id, seller_name, category_id, category_name,
                title, description, start_price, reserve_price, current_price,
                min_increment, bid_count, status, auction_id,
                starts_at, ends_at, created_at, updated_at
            "#,
        )
        .bind(seller_id)
        .bind(seller_name)
        .bind(category_id)
        .bind(category_name)
        .bind(title)
        .bind(description)
        .bind(start_price)
        .bind(reserve_price)
        .bind(start_price)   // $9 — current_price = start_price at creation
        .bind(min_increment)
        .bind(starts_at)
        .bind(ends_at)
        .fetch_one(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)
    }

    async fn get_listing(&self, id: Uuid) -> Result<Option<Listing>, ListingError> {
        sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, seller_id, seller_name, category_id, category_name,
                   title, description, start_price, reserve_price, current_price,
                   min_increment, bid_count, status, auction_id,
                   starts_at, ends_at, created_at, updated_at
            FROM listings
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)
    }

    async fn list_listings(
        &self,
        filter: ListingFilter,
    ) -> Result<(Vec<Listing>, i64), ListingError> {
        let offset = (filter.page - 1) * filter.page_size;
        // Compute pattern in Rust so both queries can borrow it
        let keyword = filter.keyword.as_ref().map(|k| format!("%{}%", k));
        let status = filter.status;

        let total: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*) FROM listings
            WHERE ($1::text IS NULL OR title ILIKE $1 OR description ILIKE $1)
              AND ($2::int4 IS NULL OR category_id = $2)
              AND ($3::int8 IS NULL OR current_price >= $3)
              AND ($4::int8 IS NULL OR current_price <= $4)
              AND ($5::timestamptz IS NULL OR ends_at <= $5)
              AND ($6::uuid IS NULL OR seller_id = $6)
              AND ($7::listing_status IS NULL OR status = $7)
            "#,
        )
        .bind(keyword.as_deref())
        .bind(filter.category_id)
        .bind(filter.min_price)
        .bind(filter.max_price)
        .bind(filter.end_before)
        .bind(filter.seller_id)
        .bind(status.as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?;

        let listings = sqlx::query_as::<_, Listing>(
            r#"
            SELECT id, seller_id, seller_name, category_id, category_name,
                   title, description, start_price, reserve_price, current_price,
                   min_increment, bid_count, status, auction_id,
                   starts_at, ends_at, created_at, updated_at
            FROM listings
            WHERE ($1::text IS NULL OR title ILIKE $1 OR description ILIKE $1)
              AND ($2::int4 IS NULL OR category_id = $2)
              AND ($3::int8 IS NULL OR current_price >= $3)
              AND ($4::int8 IS NULL OR current_price <= $4)
              AND ($5::timestamptz IS NULL OR ends_at <= $5)
              AND ($6::uuid IS NULL OR seller_id = $6)
              AND ($7::listing_status IS NULL OR status = $7)
            ORDER BY created_at DESC
            LIMIT $8 OFFSET $9
            "#,
        )
        .bind(keyword.as_deref())
        .bind(filter.category_id)
        .bind(filter.min_price)
        .bind(filter.max_price)
        .bind(filter.end_before)
        .bind(filter.seller_id)
        .bind(status.as_ref())
        .bind(filter.page_size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?;

        Ok((listings, total))
    }

    async fn update_listing(
        &self,
        id: Uuid,
        description: Option<String>,
    ) -> Result<Listing, ListingError> {
        sqlx::query_as::<_, Listing>(
            r#"
            UPDATE listings
            SET
                description = COALESCE($2, description),
                updated_at  = NOW()
            WHERE id = $1
            RETURNING
                id, seller_id, seller_name, category_id, category_name,
                title, description, start_price, reserve_price, current_price,
                min_increment, bid_count, status, auction_id,
                starts_at, ends_at, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(description)
        .fetch_optional(&self.pool)
        .await
        .map_err(ListingError::DatabaseError)?
        .ok_or(ListingError::NotFound)
    }

    async fn cancel_listing(&self, id: Uuid) -> Result<(), ListingError> {
        let affected = sqlx::query(
            r#"
            UPDATE listings
            SET status = 'CANCELLED', updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
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
