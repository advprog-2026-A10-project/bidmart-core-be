use thiserror::Error;

#[derive(Error, Debug)]
pub enum WalletError {
    #[error("Wallet not found")]
    NotFound,
    
    #[error("Insufficient balance")]
    InsufficientBalance,
    
    #[error("Invalid amount")]
    InvalidAmount,
    
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for WalletError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            WalletError::NotFound => (axum::http::StatusCode::NOT_FOUND, self.to_string()),
            WalletError::InsufficientBalance | WalletError::InvalidAmount => (
                axum::http::StatusCode::BAD_REQUEST,
                self.to_string(),
            ),
            WalletError::DatabaseError(_) | WalletError::InternalError(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = axum::Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}
