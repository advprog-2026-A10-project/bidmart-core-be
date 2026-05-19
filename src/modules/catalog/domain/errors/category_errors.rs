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
        match self {
            CategoryError::ValidationError(message) => {
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
                    CategoryError::NotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Category not found".to_string(),
                    ),
                    CategoryError::ParentNotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Parent category not found".to_string(),
                    ),
                    CategoryError::AlreadyExists => (
                        axum::http::StatusCode::CONFLICT,
                        "Category already exists".to_string(),
                    ),
                    CategoryError::DatabaseError(_) | CategoryError::InternalError(_) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    ),
                    CategoryError::ValidationError(_) => unreachable!(),
                };
                let body = axum::Json(serde_json::json!({ "message": message }));
                (status, body).into_response()
            }
        }
    }
}
