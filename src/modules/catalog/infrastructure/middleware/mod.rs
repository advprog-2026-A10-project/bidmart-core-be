use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::infrastructure::auth::{validate_session_with_auth_service, AuthValidationError};
use crate::modules::catalog::infrastructure::AppState;

#[derive(Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    message: String,
}

pub async fn require_auth(State(state): State<AppState>, mut req: Request, next: Next) -> Response {
    let authorization_header = req
        .headers()
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    let cookie_header = req
        .headers()
        .get("cookie")
        .and_then(|value| value.to_str().ok());

    let validated = match validate_session_with_auth_service(
        &state.auth_http_client,
        &state.auth_base_url,
        authorization_header,
        cookie_header,
    )
    .await
    {
        Ok(session) => session,
        Err(AuthValidationError::Unauthorized) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ErrorEnvelope {
                    message: "Unauthorized".to_string(),
                }),
            )
                .into_response();
        }
        Err(AuthValidationError::Upstream(message)) => {
            tracing::warn!("auth validate upstream error: {message}");
            return (
                StatusCode::BAD_GATEWAY,
                Json(ErrorEnvelope {
                    message: "Authentication service unavailable".to_string(),
                }),
            )
                .into_response();
        }
    };

    req.extensions_mut().insert(AuthUser {
        id: validated.user_id,
        name: validated.name,
    });
    next.run(req).await
}
