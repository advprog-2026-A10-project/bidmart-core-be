use std::sync::Arc;
use async_trait::async_trait;

use bidmart_core_be::modules::catalog::application::dto::CreateCategoryDto;
use bidmart_core_be::modules::catalog::application::use_cases::CreateCategoryUseCase;
use bidmart_core_be::modules::catalog::domain::entities::Category;
use bidmart_core_be::modules::catalog::domain::errors::CatalogError;
use bidmart_core_be::modules::catalog::domain::traits::CategoryRepository;

mod mocks {
    use super::*;

    pub struct MockCategoryRepository {
        categories: std::sync::Mutex<Vec<Category>>,
    }

    impl MockCategoryRepository {
        pub fn new() -> Self {
            Self {
                categories: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl CategoryRepository for MockCategoryRepository {
        async fn find_by_id(&self, _id: i32) -> Result<Option<Category>, CatalogError> {
            Ok(None)
        }

        async fn find_all(&self) -> Result<Vec<Category>, CatalogError> {
            Ok(self.categories.lock().unwrap().clone())
        }

        async fn find_by_slug(&self, slug: &str) -> Result<Option<Category>, CatalogError> {
            let categories = self.categories.lock().unwrap();
            Ok(categories.iter().find(|c| c.slug == slug).cloned())
        }

        async fn create(&self, category: Category) -> Result<Category, CatalogError> {
            self.categories.lock().unwrap().push(category.clone());
            Ok(category)
        }

        async fn update(&self, category: Category) -> Result<Category, CatalogError> {
            let mut categories = self.categories.lock().unwrap();
            if let Some(idx) = categories.iter().position(|c| c.id == category.id) {
                categories[idx] = category.clone();
                Ok(category)
            } else {
                Err(CatalogError::CategoryNotFound)
            }
        }

        async fn delete(&self, _id: i32) -> Result<(), CatalogError> {
            Ok(())
        }
    }
}

#[tokio::test]
async fn test_create_category_use_case() {
    use mocks::MockCategoryRepository;

    let repository = Arc::new(MockCategoryRepository::new());
    let use_case = CreateCategoryUseCase::new(repository);

    let dto = CreateCategoryDto {
        name: "Electronics".to_string(),
        slug: "electronics".to_string(),
        parent_id: None,
    };

    let result = use_case.execute(dto).await;

    assert!(result.is_ok());
    let category = result.unwrap();
    assert_eq!(category.name, "Electronics");
    assert_eq!(category.slug, "electronics");
}

#[tokio::test]
async fn test_create_category_with_duplicate_slug() {
    use mocks::MockCategoryRepository;

    // Pre-populate with a category
    let repository = Arc::new(MockCategoryRepository::new());
    
    // Manually add a category to the mock
    {
        let mut categories = std::sync::Mutex::new(Vec::new());
        categories.lock().unwrap().push(Category::new(
            "Electronics".to_string(),
            "electronics".to_string(),
            None,
        ));
    }

    let use_case = CreateCategoryUseCase::new(repository);

    let dto = CreateCategoryDto {
        name: "Electronics Duplicate".to_string(),
        slug: "electronics".to_string(),
        parent_id: None,
    };

    let result = use_case.execute(dto).await;

    assert!(result.is_err());
}
