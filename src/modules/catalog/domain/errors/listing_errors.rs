use thiserror::Error;

#[derive(Error, Debug)]
pub enum ListingError {
    #[error("Listing not found")]
    NotFound,

    #[error("Listing cannot be edited in its current state")]
    NotEditable,

    #[error("Listing cannot be cancelled in its current state")]
    NotCancellable,

    #[error("Listing cannot be published in its current state")]
    NotPublishable,

    #[error("Listing is not active")]
    #[allow(dead_code)]
    NotActive,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error("Unauthorized")]
    Unauthorized,
}

impl axum::response::IntoResponse for ListingError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ListingError::ValidationError(message) => {
                let body = axum::Json(serde_json::json!({
                    "message": "Validation failed",
                    "errors": {
                        "request": [message]
                    }
                }));
                (axum::http::StatusCode::UNPROCESSABLE_ENTITY, body).into_response()
            }
            other => {
                let (status, message) = match other {
                    ListingError::NotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Listing not found".to_string(),
                    ),
                    ListingError::NotEditable => (
                        axum::http::StatusCode::CONFLICT,
                        "Listing cannot be edited in its current state".to_string(),
                    ),
                    ListingError::NotCancellable => (
                        axum::http::StatusCode::CONFLICT,
                        "Listing cannot be cancelled in its current state".to_string(),
                    ),
                    ListingError::NotPublishable => (
                        axum::http::StatusCode::CONFLICT,
                        "Listing cannot be published in its current state".to_string(),
                    ),
                    ListingError::NotActive => (
                        axum::http::StatusCode::BAD_REQUEST,
                        "Listing is not active".to_string(),
                    ),
                    ListingError::Unauthorized => (
                        axum::http::StatusCode::FORBIDDEN,
                        "Unauthorized".to_string(),
                    ),
                    ListingError::DatabaseError(_) | ListingError::InternalError(_) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    ),
                    ListingError::ValidationError(_) => unreachable!(),
                };
                let body = axum::Json(serde_json::json!({ "message": message }));
                (status, body).into_response()
            }
        }
    }
}
