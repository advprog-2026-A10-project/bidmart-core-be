use reqwest::Client;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatedSession {
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub email_verified: bool,
    pub mfa_satisfied: bool,
    pub session_expiry: String,
}

#[derive(Debug)]
pub enum AuthValidationError {
    Unauthorized,
    Upstream(String),
}

pub async fn validate_session_with_auth_service(
    client: &Client,
    auth_base_url: &str,
    authorization_header: Option<&str>,
    cookie_header: Option<&str>,
) -> Result<ValidatedSession, AuthValidationError> {
    let validate_url = format!("{}/auth/validate", auth_base_url.trim_end_matches('/'));
    let mut request = client
        .post(validate_url)
        .header("accept", "application/json");

    if let Some(authorization) = authorization_header {
        request = request.header("authorization", authorization);
    }
    if let Some(cookie) = cookie_header {
        request = request.header("cookie", cookie);
    }

    let response = request
        .send()
        .await
        .map_err(|error| AuthValidationError::Upstream(error.to_string()))?;

    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err(AuthValidationError::Unauthorized);
    }
    if !response.status().is_success() {
        return Err(AuthValidationError::Upstream(format!(
            "auth validate returned {}",
            response.status()
        )));
    }

    response
        .json::<ValidatedSession>()
        .await
        .map_err(|error| AuthValidationError::Upstream(error.to_string()))
}
