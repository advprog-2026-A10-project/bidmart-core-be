use std::sync::Arc;

use crate::modules::catalog::domain::errors::CategoryError;
use crate::modules::catalog::domain::traits::CategoryRepository;
use crate::modules::catalog::application::dto::category_dto::CategoryResponse;

pub struct CategoryUseCases {
    category_repo: Arc<dyn CategoryRepository>,
}

impl CategoryUseCases {
    pub fn new(category_repo: Arc<dyn CategoryRepository>) -> Self {
        Self { category_repo }
    }

    pub async fn list_categories(
        &self,
        parent_id: Option<i32>,
    ) -> Result<Vec<CategoryResponse>, CategoryError> {
        let categories = self.category_repo.list_categories(parent_id).await?;
        Ok(categories.into_iter().map(CategoryResponse::from).collect())
    }

    pub async fn get_category_by_id(
        &self,
        id: i32,
    ) -> Result<CategoryResponse, CategoryError> {
        self.category_repo
            .get_category(id)
            .await?
            .map(CategoryResponse::from)
            .ok_or(CategoryError::NotFound)
    }

    pub async fn get_category_by_slug(
        &self,
        slug: &str,
    ) -> Result<CategoryResponse, CategoryError> {
        self.category_repo
            .get_category_by_slug(slug)
            .await?
            .map(CategoryResponse::from)
            .ok_or(CategoryError::NotFound)
    }
}
