use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::catalog::domain::entities::ListingStatus;
use crate::modules::catalog::infrastructure::AppState;

#[derive(Serialize)]
pub struct ListingStatusResponse {
    pub id: Uuid,
    pub status: Option<ListingStatus>,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: ListingStatus,
}

#[derive(Deserialize)]
pub struct LinkAuctionRequest {
    pub auction_id: Uuid,
}

#[derive(Deserialize)]
pub struct UpdateEndsAtRequest {
    pub new_ends_at: DateTime<Utc>,
}

// GET /internal/listings/:id/status
pub async fn get_listing_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match state.integration_service.get_listing_status(id).await {
        Ok(status) => Json(ListingStatusResponse { id, status }).into_response(),
        Err(e) => e.into_response(),
    }
}

// PATCH /internal/listings/:id/status
pub async fn update_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateStatusRequest>,
) -> impl IntoResponse {
    match state
        .integration_service
        .update_listing_status(id, body.status)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// PATCH /internal/listings/:id/auction
pub async fn link_auction(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<LinkAuctionRequest>,
) -> impl IntoResponse {
    match state
        .integration_service
        .link_auction(id, body.auction_id)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}

// PATCH /internal/listings/:id/ends-at
pub async fn update_ends_at(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateEndsAtRequest>,
) -> impl IntoResponse {
    match state
        .integration_service
        .update_ends_at(id, body.new_ends_at)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => e.into_response(),
    }
}
