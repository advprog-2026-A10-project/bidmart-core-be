// The middleware will validate JWT tokens from the Authorization header
// by calling the bidmart-auth-be service at /api/v1/auth/validate

use axum::{extract::Request, middleware::Next, response::Response};
use uuid::Uuid;

#[derive(Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub name: String,
}

// TODO: Replace with real HTTP call to auth service
pub async fn require_auth(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get("x-debug-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| Uuid::parse_str(v).ok())
        .unwrap_or_else(|| Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap());

    let name = req
        .headers()
        .get("x-debug-user-name")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Test Seller")
        .to_string();

    req.extensions_mut().insert(AuthUser { id, name });
    next.run(req).await
}
