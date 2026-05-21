use axum::async_trait;
use axum::extract::{FromRequestParts, Request};
use axum::http::{HeaderMap, HeaderValue, StatusCode, request::Parts};
use axum::middleware::Next;
use axum::response::Response;
use std::time::Instant;
use uuid::Uuid;

use crate::infrastructure::auth::{AuthValidationError, validate_session_with_auth_service};
use crate::modules::order::infrastructure::AppState;

const MODULE: &str = "order";
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_REQUEST_ID_LEN: usize = 128;

pub enum OptionalAuthError {
    Unauthorized,
    Upstream(String),
}

pub async fn resolve_authenticated_user_id(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<Uuid>, OptionalAuthError> {
    if state.auth_base_url.trim().is_empty() {
        return Ok(None);
    }

    let authorization_header = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok());
    let cookie_header = headers.get("cookie").and_then(|value| value.to_str().ok());

    if authorization_header.is_none() && cookie_header.is_none() {
        return Ok(None);
    }

    let validated = validate_session_with_auth_service(
        &state.auth_http_client,
        &state.auth_base_url,
        authorization_header,
        cookie_header,
    )
    .await
    .map_err(|error| match error {
        AuthValidationError::Unauthorized => OptionalAuthError::Unauthorized,
        AuthValidationError::Upstream(message) => OptionalAuthError::Upstream(message),
    })?;

    Ok(Some(validated.user_id))
}

/// Extractor for internal cross-module endpoints (e.g. `/events/notifications`).
/// Requires the `X-Internal-Secret` header to match the value configured via
/// `APP_ORDER_INTERNAL_SECRET`. When the env var is not set, falls back to a
/// permissive mode and logs a warning — intended for local dev only.
pub struct InternalAuth;

#[async_trait]
impl FromRequestParts<AppState> for InternalAuth {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let Some(expected) = state.internal_secret.as_deref() else {
            tracing::warn!(
                "APP_ORDER_INTERNAL_SECRET is not configured; \
                 /events/* is unauthenticated (dev-only mode)."
            );
            return Ok(InternalAuth);
        };

        let provided = parts
            .headers
            .get("x-internal-secret")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .unwrap_or_default();

        // Constant-time comparison — walks the longer of the two strings so
        // length and prefix do not leak via timing.
        let expected_bytes = expected.as_bytes();
        let provided_bytes = provided.as_bytes();
        let mut diff = (expected_bytes.len() ^ provided_bytes.len()) as u32;
        for i in 0..expected_bytes.len().max(provided_bytes.len()) {
            let a = expected_bytes.get(i).copied().unwrap_or(0);
            let b = provided_bytes.get(i).copied().unwrap_or(0);
            diff |= u32::from(a ^ b);
        }
        if diff == 0 {
            Ok(InternalAuth)
        } else {
            Err((StatusCode::UNAUTHORIZED, "Invalid internal secret"))
        }
    }
}

/// Per-module request tracer. Same shape as the catalog/bidding/wallet
/// versions so `RUST_LOG=core_be.order.request=info,...` filters
/// order-and-notification traffic per module.
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
        target: "core_be.order.request",
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
            target: "core_be.order.request",
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
            target: "core_be.order.request",
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
            target: "core_be.order.request",
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
