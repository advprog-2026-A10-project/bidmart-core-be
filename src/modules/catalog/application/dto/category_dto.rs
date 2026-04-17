use serde::Serialize;

use crate::modules::catalog::domain::entities::Category;

#[derive(Debug, Serialize)]
pub struct CategoryResponse {
    pub id: i32,
    pub parent_id: Option<i32>,
    pub name: String,
    pub slug: String,
    pub image_url: Option<String>,
    pub child_count: i32,
}

impl From<Category> for CategoryResponse {
    fn from(c: Category) -> Self {
        Self {
            id: c.id,
            parent_id: c.parent_id,
            name: c.name,
            slug: c.slug,
            image_url: c.image_url,
            child_count: c.child_count,
        }
    }
}
