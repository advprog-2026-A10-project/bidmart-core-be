use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::order::application::dto::{
    GetNotificationDto, ListNotificationsDto, MarkNotificationDto,
};
use crate::modules::order::application::use_cases::{
    GetNotificationUseCase, ListNotificationsUseCase, MarkNotificationUseCase,
};
use crate::modules::order::domain::errors::NotificationError;
use crate::modules::order::infrastructure::middleware::{
    resolve_authenticated_user_id, OptionalAuthError,
};
use crate::modules::order::infrastructure::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationsQuery {
    user_id: Option<String>,
    limit: Option<u32>,
    unread_only: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NotificationsListResponse<T> {
    data: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkAsReadBody {
    actor_id: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/:notification_id", get(get_notification))
        .route("/notifications/:notification_id/read", patch(mark_as_read))
}

async fn list_notifications(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<NotificationsQuery>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let dto = ListNotificationsDto {
        user_id: authenticated_user_id
            .map(|value| value.to_string())
            .or(query.user_id),
        limit: query.limit,
        unread_only: query.unread_only.unwrap_or(false),
    };

    let use_case = ListNotificationsUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(notifications) => (
            StatusCode::OK,
            Json(NotificationsListResponse {
                data: notifications,
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to fetch notifications",
        )
            .into_response(),
    }
}

async fn get_notification(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
) -> impl IntoResponse {
    let authenticated_user_id = match resolve_authenticated_user_id(&state, &headers).await {
        Ok(value) => value,
        Err(error) => return map_optional_auth_error(error).into_response(),
    };
    if !state.auth_base_url.trim().is_empty() && authenticated_user_id.is_none() {
        return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response();
    }

    let notification_id = match Uuid::parse_str(&notification_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid notification id").into_response(),
    };
    let dto = GetNotificationDto { notification_id };

    let use_case = GetNotificationUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(notification) => {
            if let Some(user_id) = authenticated_user_id {
                let user_id_text = user_id.to_string();
                let owner = notification
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.get("userId"))
                    .and_then(|value| value.as_str());

                if owner != Some(user_id_text.as_str()) {
                    return (StatusCode::FORBIDDEN, "Forbidden").into_response();
                }
            }

            (StatusCode::OK, Json(notification)).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Notification not found").into_response(),
    }
}

async fn mark_as_read(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(notification_id): Path<String>,
    Json(body): Json<MarkAsReadBody>,
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

    let notification_id = match Uuid::parse_str(&notification_id) {
        Ok(value) => value,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid notification id").into_response(),
    };

    let actor_id = actor_id.expect("checked actor id exists");
    let guard_use_case = GetNotificationUseCase::new(state.notification_repo.clone());
    match guard_use_case
        .execute(GetNotificationDto { notification_id })
        .await
    {
        Ok(notification) => {
            let owner = notification
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("userId"))
                .and_then(|value| value.as_str());

            if owner != Some(actor_id.as_str()) {
                return (StatusCode::FORBIDDEN, "Forbidden").into_response();
            }
        }
        Err(error) => {
            return map_notification_error(error, "Unable to load notification").into_response();
        }
    }

    let dto = MarkNotificationDto {
        notification_id,
        actor_id,
    };
    let use_case = MarkNotificationUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => map_notification_error(error, "Unable to mark notification").into_response(),
    }
}

fn map_notification_error(
    error: NotificationError,
    fallback_message: &'static str,
) -> (StatusCode, &'static str) {
    match error {
        NotificationError::NotFound => (StatusCode::NOT_FOUND, "Notification not found"),
        NotificationError::AlreadyRead => {
            (StatusCode::CONFLICT, "Notification already marked as read")
        }
        NotificationError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, fallback_message),
    }
}

fn map_optional_auth_error(error: OptionalAuthError) -> (StatusCode, &'static str) {
    match error {
        OptionalAuthError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
        OptionalAuthError::Upstream(message) => {
            tracing::warn!("order notification auth validate upstream error: {message}");
            (
                StatusCode::BAD_GATEWAY,
                "Authentication service unavailable",
            )
        }
    }
}
