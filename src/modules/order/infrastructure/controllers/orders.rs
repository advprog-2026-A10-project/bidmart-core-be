use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::order::application::dto::{
    ConfirmOrderDto, CreateDisputeDto, GetOrderDto, ListOrdersDto, UpdateShippingDto,
};
use crate::modules::order::application::use_cases::{
    ConfirmOrderUseCase, CreateDisputeUseCase, GetOrderUseCase, ListOrdersUseCase,
    UpdateShippingUseCase,
};
use crate::modules::order::domain::entities::OrderStage;
use crate::modules::order::infrastructure::AppState;

#[derive(Deserialize)]
struct ListOrdersQuery {
    role: Option<String>,
    stage: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_orders))
        .route("/orders/:order_id", get(get_order))
        .route("/orders/:order_id/confirm", post(confirm_order))
        .route("/orders/:order_id/dispute/new", post(create_dispute))
        .route("/seller/orders", get(list_orders))
        .route("/seller/orders/:order_id", get(get_order))
        .route("/seller/orders/:order_id/shipping", patch(update_shipping))
}

async fn list_orders(
    State(state): State<AppState>,
    Query(query): Query<ListOrdersQuery>,
) -> impl IntoResponse {
    let stage = query.stage.as_deref().and_then(parse_stage);

    let dto = ListOrdersDto {
        user_id: None,
        role: query.role.unwrap_or_else(|| "buyer".to_string()),
        stage,
    };

    let use_case = ListOrdersUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(orders) => (StatusCode::OK, Json(orders)).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to list orders").into_response(),
    }
}

async fn get_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let order_id = Uuid::parse_str(&order_id).unwrap_or_default();
    let dto = GetOrderDto { order_id };

    let use_case = GetOrderUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(order) => (StatusCode::OK, Json(order)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Order not found").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ConfirmOrderBody {
    actor_id: String,
}

async fn confirm_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(body): Json<ConfirmOrderBody>,
) -> impl IntoResponse {
    let order_id = Uuid::parse_str(&order_id).unwrap_or_default();
    let dto = ConfirmOrderDto {
        order_id,
        actor_id: body.actor_id,
    };

    let use_case = ConfirmOrderUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to confirm order").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct CreateDisputeBody {
    reporter_id: String,
    reason: String,
}

async fn create_dispute(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(body): Json<CreateDisputeBody>,
) -> impl IntoResponse {
    let order_id = Uuid::parse_str(&order_id).unwrap_or_default();
    let dto = CreateDisputeDto {
        order_id,
        reporter_id: body.reporter_id,
        reason: body.reason,
    };

    let use_case = CreateDisputeUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to create dispute",
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct UpdateShippingBody {
    status: String,
    tracking: Option<String>,
}

async fn update_shipping(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(body): Json<UpdateShippingBody>,
) -> impl IntoResponse {
    let order_id = Uuid::parse_str(&order_id).unwrap_or_default();
    let dto = UpdateShippingDto {
        order_id,
        status: body.status,
        tracking: body.tracking,
    };

    let use_case = UpdateShippingUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to update shipping",
        )
            .into_response(),
    }
}

fn parse_stage(input: &str) -> Option<OrderStage> {
    match input.to_lowercase().as_str() {
        "active" => Some(OrderStage::Active),
        "processing" => Some(OrderStage::Processing),
        "completed" => Some(OrderStage::Completed),
        "cancelled" => Some(OrderStage::Cancelled),
        _ => None,
    }
}
