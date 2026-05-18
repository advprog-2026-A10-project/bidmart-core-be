use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};

use crate::modules::order::application::dto::PublishEventDto;
use crate::modules::order::application::use_cases::PublishEventUseCase;
use crate::modules::order::domain::entities::NotificationEventPayload;
use crate::modules::order::infrastructure::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/events/notifications", post(publish_event))
}

async fn publish_event(
    State(state): State<AppState>,
    Json(payload): Json<NotificationEventPayload>,
) -> impl IntoResponse {
    let dto = PublishEventDto { payload };
    let use_case = PublishEventUseCase::new(state.notification_repo.clone());

    match use_case.execute(dto).await {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Unable to publish event").into_response(),
    }
}
