use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::modules::catalog::application::dto::category_dto::CategoryResponse;
use crate::modules::catalog::domain::errors::CategoryError;
use crate::modules::catalog::infrastructure::AppState;

#[derive(Deserialize)]
pub struct CategoryQueryParams {
    pub parent_id: Option<i32>,
}

// GET /api/v1/categories
pub async fn list_categories(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<CategoryQueryParams>,
) -> Result<Json<Vec<CategoryResponse>>, CategoryError> {
    let result = state.category_use_cases.list_categories(params.parent_id).await?;
    Ok(Json(result))
}

// GET /api/v1/categories/:id
pub async fn get_category_by_id(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<CategoryResponse>, CategoryError> {
    let result = state.category_use_cases.get_category_by_id(id).await?;
    Ok(Json(result))
}
