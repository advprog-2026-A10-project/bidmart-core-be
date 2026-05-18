use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::modules::catalog::application::dto::listing_dto::{
    CreateListingRequest, ListingDetailResponse, ListingQueryParams,
    ListingResponse, PaginatedResponse, UpdateListingRequest,
};
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::infrastructure::middleware::AuthUser;
use crate::modules::catalog::infrastructure::AppState;

// GET /api/v1/seller/listings
pub async fn list_my_listings(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    axum::extract::Query(params): axum::extract::Query<ListingQueryParams>,
) -> Result<Json<PaginatedResponse<ListingResponse>>, ListingError> {
    let result = state.listing_use_cases.list_my_listings(auth.id, params).await?;
    Ok(Json(result))
}

// POST /api/v1/seller/listings
pub async fn create_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Json(body): Json<CreateListingRequest>,
) -> Result<(StatusCode, Json<ListingDetailResponse>), ListingError> {
    let result = state.listing_use_cases.create_listing(auth.id, auth.name, body).await?;
    Ok((StatusCode::CREATED, Json(result)))
}

// GET /api/v1/seller/listings/:id
pub async fn get_my_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListingDetailResponse>, ListingError> {
    let result = state.listing_use_cases.get_my_listing(auth.id, id).await?;
    Ok(Json(result))
}

// PATCH /api/v1/seller/listings/:id
pub async fn update_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateListingRequest>,
) -> Result<Json<ListingDetailResponse>, ListingError> {
    let result = state.listing_use_cases.update_listing(auth.id, id, body).await?;
    Ok(Json(result))
}

// DELETE /api/v1/seller/listings/:id
pub async fn cancel_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ListingError> {
    state.listing_use_cases.cancel_listing(auth.id, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn publish_listing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<ListingDetailResponse>, ListingError> {
    let result = state.listing_use_cases.publish_listing(auth.id, id).await?;
    Ok(Json(result))
}
