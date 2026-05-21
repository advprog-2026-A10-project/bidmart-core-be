use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request::Parts, HeaderMap, StatusCode};
use uuid::Uuid;

use crate::infrastructure::auth::{validate_session_with_auth_service, AuthValidationError};
use crate::modules::order::infrastructure::AppState;

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
