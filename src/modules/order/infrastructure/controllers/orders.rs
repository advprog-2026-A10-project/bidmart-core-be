use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

use crate::modules::order::application::dto::{
    ConfirmOrderDto, CreateDisputeDto, GetOrderDto, ListOrdersDto, UpdateShippingDto,
};
use crate::modules::order::application::use_cases::{
    ConfirmOrderUseCase, CreateDisputeUseCase, GetOrderUseCase, ListOrdersUseCase,
    UpdateShippingUseCase,
};
use crate::modules::order::domain::entities::OrderStage;
use crate::modules::order::domain::errors::OrderError;
use crate::modules::order::infrastructure::AppState;

#[derive(Deserialize)]
struct ListOrdersQuery {
    #[serde(rename = "userId")]
    user_id: Option<String>,
    stage: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrdersListResponse<T> {
    data: T,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list_buyer_orders))
        .route("/orders/:order_id", get(get_order))
        .route("/orders/:order_id/confirm", post(confirm_order))
        .route("/orders/:order_id/dispute/new", post(create_dispute))
        .route("/seller/orders", get(list_seller_orders))
        .route("/seller/orders/:order_id", get(get_order))
        .route("/seller/orders/:order_id/shipping", patch(update_shipping))
}

async fn list_buyer_orders(
    State(state): State<AppState>,
    Query(query): Query<ListOrdersQuery>,
) -> impl IntoResponse {
    list_orders_by_role(state, query, "buyer").await
}

async fn list_seller_orders(
    State(state): State<AppState>,
    Query(query): Query<ListOrdersQuery>,
) -> impl IntoResponse {
    list_orders_by_role(state, query, "seller").await
}

async fn list_orders_by_role(
    state: AppState,
    query: ListOrdersQuery,
    role: &str,
) -> impl IntoResponse {
    let stage = match query.stage.as_deref() {
        Some(stage_value) => match parse_stage(stage_value) {
            Some(parsed_stage) => Some(parsed_stage),
            None => return (StatusCode::BAD_REQUEST, "Invalid stage").into_response(),
        },
        None => None,
    };

    let dto = ListOrdersDto {
        user_id: query.user_id,
        role: role.to_string(),
        stage,
    };

    let use_case = ListOrdersUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(orders) => (StatusCode::OK, Json(OrdersListResponse { data: orders })).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to list orders").into_response(),
    }
}

async fn get_order(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let parsed_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = GetOrderDto {
        order_id: parsed_id,
    };

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
    if body.actor_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "actor_id is required").into_response();
    }

    let order_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = ConfirmOrderDto {
        order_id,
        actor_id: body.actor_id,
    };

    let use_case = ConfirmOrderUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_order_error(error, "Unable to confirm order").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct CreateDisputeBody {
    reporter_id: String,
    reason: String,
    details: Option<String>,
}

async fn create_dispute(
    State(state): State<AppState>,
    Path(order_id): Path<String>,
    Json(body): Json<CreateDisputeBody>,
) -> impl IntoResponse {
    if body.reporter_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "reporter_id is required").into_response();
    }

    if body.reason.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "reason is required").into_response();
    }

    if !body.reason.to_lowercase().contains("barang tidak sesuai") {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            "reason must include 'barang tidak sesuai'",
        )
            .into_response();
    }

    let order_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = CreateDisputeDto {
        order_id,
        reporter_id: body.reporter_id,
        reason: body.reason,
        details: body.details,
    };

    let use_case = CreateDisputeUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_order_error(error, "Unable to create dispute").into_response(),
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
    if body.status.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "status is required").into_response();
    }

    let order_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = UpdateShippingDto {
        order_id,
        status: body.status,
        tracking: body.tracking,
    };

    let use_case = UpdateShippingUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_order_error(error, "Unable to update shipping").into_response(),
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

fn map_order_error(error: OrderError, fallback_message: &'static str) -> (StatusCode, &'static str) {
    match error {
        OrderError::NotFound => (StatusCode::NOT_FOUND, "Order not found"),
        OrderError::InvalidTransition => (StatusCode::CONFLICT, "Invalid order transition"),
        OrderError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, fallback_message),
    }
}
