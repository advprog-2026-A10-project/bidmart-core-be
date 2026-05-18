use async_trait::async_trait;

use crate::modules::catalog::domain::entities::Category;
use crate::modules::catalog::domain::errors::CategoryError;

#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn get_category(
        &self,
        id: i32,
    ) -> Result<Option<Category>, CategoryError>;

    async fn get_category_by_slug(
        &self,
        slug: &str,
    ) -> Result<Option<Category>, CategoryError>;

    async fn get_subtree_ids(
        &self,
        root_id: i32,
    ) -> Result<Vec<i32>, CategoryError>;

    async fn list_categories(
        &self,
        parent_id: Option<i32>,
    ) -> Result<Vec<Category>, CategoryError>;

    async fn create_category(
        &self,
        name: String,
        slug: String,
        parent_id: Option<i32>,
        image_url: Option<String>,
    ) -> Result<Category, CategoryError>;

    async fn update_category(
        &self,
        id: i32,
        name: Option<String>,
        slug: Option<String>,
        image_url: Option<String>,
    ) -> Result<Category, CategoryError>;

    async fn delete_category(
        &self,
        id: i32,
    ) -> Result<(), CategoryError>;
}