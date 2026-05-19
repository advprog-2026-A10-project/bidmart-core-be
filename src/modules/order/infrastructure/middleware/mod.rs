use axum::http::HeaderMap;
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
