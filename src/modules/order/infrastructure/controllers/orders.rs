use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
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
use crate::modules::order::infrastructure::middleware::{
    resolve_authenticated_user_id, OptionalAuthError,
};
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
    headers: HeaderMap,
    Query(query): Query<ListOrdersQuery>,
) -> Response {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    list_orders_by_role(state, query, "buyer", authenticated_user_id).await
}

async fn list_seller_orders(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListOrdersQuery>,
) -> Response {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    list_orders_by_role(state, query, "seller", authenticated_user_id).await
}

async fn list_orders_by_role(
    state: AppState,
    query: ListOrdersQuery,
    role: &str,
    authenticated_user_id: Option<Uuid>,
) -> Response {
    let stage = match query.stage.as_deref() {
        Some(stage_value) => match parse_stage(stage_value) {
            Some(parsed_stage) => Some(parsed_stage),
            None => return (StatusCode::BAD_REQUEST, "Invalid stage").into_response(),
        },
        None => None,
    };

    let dto = ListOrdersDto {
        user_id: authenticated_user_id
            .map(|value| value.to_string())
            .or(query.user_id),
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
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let parsed_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = GetOrderDto {
        order_id: parsed_id,
    };

    let use_case = GetOrderUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(order) => {
            if let Some(user_id) = authenticated_user_id {
                let user_id_text = user_id.to_string();
                if order.buyer_id != user_id_text && order.seller_id != user_id_text {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }

            (StatusCode::OK, Json(order)).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Order not found").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct ConfirmOrderBody {
    actor_id: Option<String>,
}

async fn confirm_order(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(body): Json<ConfirmOrderBody>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let actor_id = authenticated_user_id
        .map(|value| value.to_string())
        .or(body.actor_id.and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }));

    if actor_id.is_none() {
        return (StatusCode::BAD_REQUEST, "actor_id is required").into_response();
    }

    let order_id = match Uuid::parse_str(&order_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid order id").into_response(),
    };
    let dto = ConfirmOrderDto {
        order_id,
        actor_id: actor_id.expect("checked actor id exists"),
    };

    if let Some(user_id) = authenticated_user_id {
        let guard_use_case = GetOrderUseCase::new(state.order_repo.clone());
        match guard_use_case.execute(GetOrderDto { order_id }).await {
            Ok(order) => {
                if order.buyer_id != user_id.to_string() {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }
            Err(_) => return (StatusCode::NOT_FOUND, "Order not found").into_response(),
        }
    }

    let use_case = ConfirmOrderUseCase::new(state.order_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_order_error(error, "Unable to confirm order").into_response(),
    }
}

#[derive(serde::Deserialize)]
struct CreateDisputeBody {
    reporter_id: Option<String>,
    reason: String,
    details: Option<String>,
}

async fn create_dispute(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(body): Json<CreateDisputeBody>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let reporter_id = authenticated_user_id.map(|value| value.to_string()).or(body
        .reporter_id
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }));

    if reporter_id.is_none() {
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
        reporter_id: reporter_id.expect("checked reporter id exists"),
        reason: body.reason,
        details: body.details,
    };

    if let Some(user_id) = authenticated_user_id {
        let guard_use_case = GetOrderUseCase::new(state.order_repo.clone());
        match guard_use_case.execute(GetOrderDto { order_id }).await {
            Ok(order) => {
                if order.buyer_id != user_id.to_string() {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }
            Err(_) => return (StatusCode::NOT_FOUND, "Order not found").into_response(),
        }
    }

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
    headers: HeaderMap,
    Path(order_id): Path<String>,
    Json(body): Json<UpdateShippingBody>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

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

    if let Some(user_id) = authenticated_user_id {
        let guard_use_case = GetOrderUseCase::new(state.order_repo.clone());
        match guard_use_case.execute(GetOrderDto { order_id }).await {
            Ok(order) => {
                if order.seller_id != user_id.to_string() {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }
            Err(_) => return (StatusCode::NOT_FOUND, "Order not found").into_response(),
        }
    }

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

fn map_order_error(
    error: OrderError,
    fallback_message: &'static str,
) -> (StatusCode, &'static str) {
    match error {
        OrderError::NotFound => (StatusCode::NOT_FOUND, "Order not found"),
        OrderError::InvalidTransition => (StatusCode::CONFLICT, "Invalid order transition"),
        OrderError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, fallback_message),
    }
}

fn map_optional_auth_error(error: OptionalAuthError) -> (StatusCode, &'static str) {
    match error {
        OptionalAuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
        OptionalAuthError::Upstream(message) => {
            tracing::warn!("order auth validate upstream error: {message}");
            (
                StatusCode::BAD_GATEWAY,
                "Authentication service unavailable",
            )
        }
    }
}
