use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

use crate::infrastructure::auth::{validate_session_with_auth_service, AuthValidationError};
use crate::modules::wallet::infrastructure::controllers::WalletAppState;

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
impl<S> FromRequestParts<S> for InternalAuth
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // MOCK INTERNAL AUTHENTICATION:
        // A real implementation would check for a shared secret header or mTLS certificate.
        Ok(InternalAuth)
    }
}
