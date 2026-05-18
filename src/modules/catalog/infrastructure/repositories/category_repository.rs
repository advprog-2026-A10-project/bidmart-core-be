use async_trait::async_trait;
use sqlx::PgPool;

use crate::modules::catalog::domain::entities::Category;
use crate::modules::catalog::domain::errors::CategoryError;
use crate::modules::catalog::domain::traits::CategoryRepository;

fn map_category_db_error(e: sqlx::Error) -> CategoryError {
    match &e {
        sqlx::Error::Database(db) if db.code().as_deref() == Some("23505") => {
            CategoryError::AlreadyExists
        }
        _ => CategoryError::DatabaseError(e),
    }
}

pub struct PostgresCategoryRepository {
    pool: PgPool,
}

impl PostgresCategoryRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CategoryRepository for PostgresCategoryRepository {
    async fn get_category(&self, id: i32) -> Result<Option<Category>, CategoryError> {
        sqlx::query_as::<_, Category>(
            r#"
            SELECT id, parent_id, name, slug, image_url, child_count, created_at, updated_at
            FROM categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(CategoryError::DatabaseError)
    }

    async fn get_category_by_slug(&self, slug: &str) -> Result<Option<Category>, CategoryError> {
        sqlx::query_as::<_, Category>(
            r#"
            SELECT id, parent_id, name, slug, image_url, child_count, created_at, updated_at
            FROM categories
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .map_err(CategoryError::DatabaseError)
    }

    async fn list_categories(
        &self,
        parent_id: Option<i32>,
    ) -> Result<Vec<Category>, CategoryError> {
        // None → return root categories (parent_id IS NULL)
        // Some(id) → return direct children of that category
        sqlx::query_as::<_, Category>(
            r#"
            SELECT id, parent_id, name, slug, image_url, child_count, created_at, updated_at
            FROM categories
            WHERE ($1::int4 IS NULL AND parent_id IS NULL)
               OR (parent_id = $1)
            ORDER BY name ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(CategoryError::DatabaseError)
    }

    async fn create_category(
        &self,
        name: String,
        slug: String,
        parent_id: Option<i32>,
        image_url: Option<String>,
    ) -> Result<Category, CategoryError> {
        let mut tx = self.pool.begin().await.map_err(CategoryError::DatabaseError)?;

        let category = sqlx::query_as::<_, Category>(
            r#"
            INSERT INTO categories (name, slug, parent_id, image_url)
            VALUES ($1, $2, $3, $4)
            RETURNING id, parent_id, name, slug, image_url, child_count, created_at, updated_at
            "#,
        )
        .bind(&name)
        .bind(&slug)
        .bind(parent_id)
        .bind(image_url)
        .fetch_one(&mut *tx)
        .await
        .map_err(map_category_db_error)?;

        // Keep parent's child_count in sync
        if let Some(pid) = parent_id {
            sqlx::query(
                "UPDATE categories SET child_count = child_count + 1, updated_at = NOW() WHERE id = $1",
            )
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(CategoryError::DatabaseError)?;
        }

        tx.commit().await.map_err(CategoryError::DatabaseError)?;
        Ok(category)
    }

    async fn update_category(
        &self,
        id: i32,
        name: Option<String>,
        slug: Option<String>,
        image_url: Option<String>,
    ) -> Result<Category, CategoryError> {
        sqlx::query_as::<_, Category>(
            r#"
            UPDATE categories
            SET
                name       = COALESCE($2, name),
                slug       = COALESCE($3, slug),
                image_url  = COALESCE($4, image_url),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, parent_id, name, slug, image_url, child_count, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(slug)
        .bind(image_url)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_category_db_error)?
        .ok_or(CategoryError::NotFound)
    }

    async fn delete_category(&self, id: i32) -> Result<(), CategoryError> {
        let mut tx = self.pool.begin().await.map_err(CategoryError::DatabaseError)?;

        // Fetch parent_id first so we can decrement its child_count after deletion
        let parent_id: Option<i32> = sqlx::query_scalar(
            "SELECT parent_id FROM categories WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(CategoryError::DatabaseError)?
        .flatten();

        let affected = sqlx::query("DELETE FROM categories WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .map_err(CategoryError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(CategoryError::NotFound);
        }

        if let Some(pid) = parent_id {
            sqlx::query(
                "UPDATE categories SET child_count = child_count - 1, updated_at = NOW() WHERE id = $1",
            )
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(CategoryError::DatabaseError)?;
        }

        tx.commit().await.map_err(CategoryError::DatabaseError)?;
        Ok(())
    }

    async fn get_subtree_ids(&self, root_id: i32) -> Result<Vec<i32>, CategoryError> {
        let rows: Vec<(i32,)> = sqlx::query_as(
            r#"
            WITH RECURSIVE subtree AS (
                SELECT id, 1 AS depth FROM categories WHERE id = $1
                UNION ALL
                SELECT c.id, s.depth + 1 FROM categories c
                INNER JOIN subtree s ON c.parent_id = s.id
                WHERE s.depth < 50
            )
            SELECT id FROM subtree
            "#,
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await
        .map_err(CategoryError::DatabaseError)?;

        Ok(rows.into_iter().map(|(id,)| id).collect())
    }
}
