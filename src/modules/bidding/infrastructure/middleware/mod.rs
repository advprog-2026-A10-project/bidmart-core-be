use axum::{
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::time::Instant;
use uuid::Uuid;

use crate::infrastructure::auth::{validate_session_with_auth_service, AuthValidationError};
use crate::modules::bidding::infrastructure::AppState;

const MODULE: &str = "bidding";
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_REQUEST_ID_LEN: usize = 128;

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
            tracing::warn!("bidding auth validate upstream error: {message}");
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

/// Per-module request tracer. Mirrors the catalog version so logs from each
/// module are filterable via `RUST_LOG=core_be.bidding.request=info,...`.
pub async fn request_trace(req: Request, next: Next) -> Response {
    let request_id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= MAX_REQUEST_ID_LEN)
        .map(ToString::to_string)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    tracing::info!(
        target: "core_be.bidding.request",
        request_id = %request_id,
        module = MODULE,
        method = %method,
        path = %path,
        "request_started"
    );

    let started_at = Instant::now();
    let mut response = next.run(req).await;
    let elapsed_ms = started_at.elapsed().as_millis() as u64;
    let status = response.status();
    let status_code = status.as_u16();

    if let Ok(header_value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(REQUEST_ID_HEADER, header_value);
    }

    if status.is_server_error() {
        tracing::error!(
            target: "core_be.bidding.request",
            request_id = %request_id,
            module = MODULE,
            method = %method,
            path = %path,
            status = status_code,
            elapsed_ms,
            "request_finished_with_server_error"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            target: "core_be.bidding.request",
            request_id = %request_id,
            module = MODULE,
            method = %method,
            path = %path,
            status = status_code,
            elapsed_ms,
            "request_finished_with_client_error"
        );
    } else {
        tracing::info!(
            target: "core_be.bidding.request",
            request_id = %request_id,
            module = MODULE,
            method = %method,
            path = %path,
            status = status_code,
            elapsed_ms,
            "request_finished"
        );
    }

    response
}
