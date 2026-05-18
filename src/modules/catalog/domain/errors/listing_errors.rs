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
        let (status, message) = match self {
            ListingError::NotFound => {
                (axum::http::StatusCode::NOT_FOUND, self.to_string())
            }
            ListingError::NotEditable
            | ListingError::NotCancellable
            | ListingError::NotPublishable => {
                (axum::http::StatusCode::CONFLICT, self.to_string())
            }
            ListingError::NotActive | ListingError::ValidationError(_) => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            ListingError::Unauthorized => {
                (axum::http::StatusCode::FORBIDDEN, self.to_string())
            }
            ListingError::DatabaseError(_) | ListingError::InternalError(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        let body = axum::Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}
