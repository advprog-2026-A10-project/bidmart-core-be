use axum::{
    async_trait,
    extract::{FromRequestParts, Request},
    http::{request::Parts, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use uuid::Uuid;

use crate::infrastructure::auth::{validate_session_with_auth_service, AuthValidationError};
use crate::modules::wallet::infrastructure::controllers::WalletAppState;

const MODULE: &str = "wallet";
const REQUEST_ID_HEADER: &str = "x-request-id";
const MAX_REQUEST_ID_LEN: usize = 128;

pub struct AuthUser {
    pub user_id: Uuid,
}

#[async_trait]
impl FromRequestParts<WalletAppState> for AuthUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &WalletAppState,
    ) -> Result<Self, Self::Rejection> {
        let authorization = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok());
        let cookie = parts
            .headers
            .get("cookie")
            .and_then(|value| value.to_str().ok());

        let validated = validate_session_with_auth_service(
            &state.auth_http_client,
            &state.auth_base_url,
            authorization,
            cookie,
        )
        .await
        .map_err(|error| match error {
            AuthValidationError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AuthValidationError::Upstream(message) => {
                tracing::warn!("wallet auth validate upstream error: {message}");
                (
                    StatusCode::BAD_GATEWAY,
                    "Authentication service unavailable",
                )
            }
        })?;

        Ok(AuthUser {
            user_id: validated.user_id,
        })
    }
}

pub struct InternalAuth;

#[async_trait]
impl FromRequestParts<WalletAppState> for InternalAuth {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &WalletAppState,
    ) -> Result<Self, Self::Rejection> {
        // No secret configured → fall back to permissive dev mode but log a
        // warning so it is visible in deployments where the env var was
        // forgotten.
        let Some(expected) = state.internal_secret.as_deref() else {
            tracing::warn!(
                "APP_WALLET_INTERNAL_SECRET is not configured; \
                 /internal/wallet/* is unauthenticated (dev-only mode)."
            );
            return Ok(InternalAuth);
        };

        let provided = parts
            .headers
            .get("x-internal-secret")
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .unwrap_or_default();

        // Constant-time comparison to avoid leaking secret length / prefix via
        // timing. constant_time_eq is not in deps, so use a manual loop that
        // walks the longer of the two strings.
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

/// Per-module request tracer. Same shape as the catalog/bidding versions so
/// `RUST_LOG=core_be.wallet.request=info,...` filters wallet-only traffic.
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
        target: "core_be.wallet.request",
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
            target: "core_be.wallet.request",
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
            target: "core_be.wallet.request",
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
            target: "core_be.wallet.request",
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
