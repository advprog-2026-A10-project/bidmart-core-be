use thiserror::Error;

#[derive(Error, Debug)]
pub enum CategoryError {
    #[error("Category not found")]
    NotFound,

    #[error("Parent category not found")]
    ParentNotFound,

    #[error("Category already exists")]
    AlreadyExists,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for CategoryError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            CategoryError::NotFound | CategoryError::ParentNotFound => {
                (axum::http::StatusCode::NOT_FOUND, self.to_string())
            }
            CategoryError::AlreadyExists => {
                (axum::http::StatusCode::CONFLICT, self.to_string())
            }
            CategoryError::ValidationError(_) => {
                (axum::http::StatusCode::BAD_REQUEST, self.to_string())
            }
            CategoryError::DatabaseError(_) | CategoryError::InternalError(_) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        let body = axum::Json(serde_json::json!({ "error": message }));
        (status, body).into_response()
    }
}
