use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::modules::catalog::application::dto::buyer_listing_dto::{
    BuyerListingDetailResponse, BuyerListingResponse, CatalogPageParams, PublicListingQueryParams,
};
use crate::modules::catalog::application::dto::listing_dto::PaginatedResponse;
use crate::modules::catalog::domain::errors::ListingError;
use crate::modules::catalog::infrastructure::AppState;

// GET /api/v1/catalog
pub async fn browse_catalog(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<PublicListingQueryParams>,
) -> Result<Json<PaginatedResponse<BuyerListingResponse>>, ListingError> {
    let result = state.buyer_listing_use_cases.browse_catalog(params).await?;
    Ok(Json(result))
}

// GET /api/v1/listings/:id
pub async fn get_public_listing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<BuyerListingDetailResponse>, ListingError> {
    let result = state.buyer_listing_use_cases.get_public_listing(id).await?;
    Ok(Json(result))
}

// GET /api/v1/c/*category_path
pub async fn browse_by_category_path(
    State(state): State<AppState>,
    Path(category_path): Path<String>,
    axum::extract::Query(params): axum::extract::Query<CatalogPageParams>,
) -> Result<Json<PaginatedResponse<BuyerListingResponse>>, ListingError> {
    let page = params.page.unwrap_or(1);
    let page_size = params.page_size.unwrap_or(20);
    let result = state
        .buyer_listing_use_cases
        .browse_by_category_path(&category_path, page, page_size)
        .await?;
    Ok(Json(result))
}
