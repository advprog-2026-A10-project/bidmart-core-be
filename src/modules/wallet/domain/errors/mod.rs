use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet not found")]
    NotFound,

    #[error("Insufficient balance")]
    InsufficientBalance,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for WalletError {
    fn into_response(self) -> axum::response::Response {
        match self {
            WalletError::ValidationError(message) => {
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
                    WalletError::NotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Wallet not found".to_string(),
                    ),
                    WalletError::InsufficientBalance => (
                        axum::http::StatusCode::BAD_REQUEST,
                        "Insufficient balance".to_string(),
                    ),
                    WalletError::DatabaseError(_) | WalletError::InternalError(_) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    ),
                    WalletError::ValidationError(_) => unreachable!(),
                };

                let body = axum::Json(serde_json::json!({
                    "message": message,
                }));
                (status, body).into_response()
            }
        }
    }
}
