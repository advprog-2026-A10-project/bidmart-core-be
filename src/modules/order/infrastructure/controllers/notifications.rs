use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::order::application::dto::{GetNotificationDto, ListNotificationsDto};
use crate::modules::order::application::use_cases::{
    GetNotificationUseCase, ListNotificationsUseCase,
};
use crate::modules::order::infrastructure::AppState;

#[derive(Deserialize)]
struct NotificationsQuery {
    user_id: Option<String>,
    limit: Option<u32>,
    unread_only: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/notifications", get(list_notifications))
        .route("/notifications/:notification_id", get(get_notification))
}

async fn list_notifications(
    State(state): State<AppState>,
    Query(query): Query<NotificationsQuery>,
) -> impl IntoResponse {
    let dto = ListNotificationsDto {
        user_id: query.user_id,
        limit: query.limit,
        unread_only: query.unread_only.unwrap_or(false),
    };

    let use_case = ListNotificationsUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(notifications) => (StatusCode::OK, Json(notifications)).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to fetch notifications",
        )
            .into_response(),
    }
}

async fn get_notification(
    State(state): State<AppState>,
    Path(notification_id): Path<String>,
) -> impl IntoResponse {
    let notification_id = Uuid::parse_str(&notification_id).unwrap_or_default();
    let dto = GetNotificationDto { notification_id };

    let use_case = GetNotificationUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(notification) => (StatusCode::OK, Json(notification)).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Notification not found").into_response(),
    }
}
