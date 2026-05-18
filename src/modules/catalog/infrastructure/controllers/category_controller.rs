use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::modules::catalog::application::dto::category_dto::{
    CategoryResponse, CreateCategoryRequest, UpdateCategoryRequest,
};
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

pub async fn get_category_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<CategoryResponse>, CategoryError> {
    let result = state.category_use_cases.get_category_by_slug(&slug).await?;
    Ok(Json(result))
}

pub async fn create_category(
    State(state): State<AppState>,
    Json(body): Json<CreateCategoryRequest>,
) -> Result<(StatusCode, Json<CategoryResponse>), CategoryError> {
    let result = state.category_use_cases.create_category(body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

pub async fn update_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(body): Json<UpdateCategoryRequest>,
) -> Result<Json<CategoryResponse>, CategoryError> {
    let result = state.category_use_cases.update_category(id, body).await?;
    Ok(Json(result))
}

pub async fn delete_category(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, CategoryError> {
    state.category_use_cases.delete_category(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
