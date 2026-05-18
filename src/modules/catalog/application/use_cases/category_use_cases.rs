use std::sync::Arc;

use crate::modules::catalog::domain::errors::CategoryError;
use crate::modules::catalog::domain::traits::CategoryRepository;
use crate::modules::catalog::application::dto::category_dto::{
    CategoryResponse, CreateCategoryRequest, UpdateCategoryRequest,
};

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

    pub async fn create_category(
        &self,
        req: CreateCategoryRequest,
    ) -> Result<CategoryResponse, CategoryError> {
        let name = req.name.trim().to_string();
        let slug = req.slug.trim().to_string();
        if name.is_empty() {
            return Err(CategoryError::ValidationError(
                "Name cannot be empty".to_string(),
            ));
        }
        if slug.is_empty() || slug.contains(' ') {
            return Err(CategoryError::ValidationError(
                "Slug must be non-empty and contain no spaces".to_string(),
            ));
        }
        if let Some(parent_id) = req.parent_id {
            self.category_repo
                .get_category(parent_id)
                .await?
                .ok_or(CategoryError::ParentNotFound)?;
        }

        let category = self
            .category_repo
            .create_category(name, slug, req.parent_id, req.image_url)
            .await?;
        Ok(CategoryResponse::from(category))
    }

    pub async fn update_category(
        &self,
        id: i32,
        req: UpdateCategoryRequest,
    ) -> Result<CategoryResponse, CategoryError> {
        let name = match req.name {
            Some(n) => {
                let n = n.trim().to_string();
                if n.is_empty() {
                    return Err(CategoryError::ValidationError(
                        "Name cannot be empty".to_string(),
                    ));
                }
                Some(n)
            }
            None => None,
        };
        let slug = match req.slug {
            Some(s) => {
                let s = s.trim().to_string();
                if s.is_empty() || s.contains(' ') {
                    return Err(CategoryError::ValidationError(
                        "Slug must be non-empty and contain no spaces".to_string(),
                    ));
                }
                Some(s)
            }
            None => None,
        };

        let category = self
            .category_repo
            .update_category(id, name, slug, req.image_url)
            .await?;
        Ok(CategoryResponse::from(category))
    }

    pub async fn delete_category(&self, id: i32) -> Result<(), CategoryError> {
        self.category_repo.delete_category(id).await
    }
}
