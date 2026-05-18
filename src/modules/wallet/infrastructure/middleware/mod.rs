use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use uuid::Uuid;

pub struct AuthUser {
    pub user_id: Uuid,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Mock Authentication: Return a default/dummy test user ID for now
        // Default mock UUID: 00000000-0000-0000-0000-000000000001
        Ok(AuthUser {
            user_id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
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
