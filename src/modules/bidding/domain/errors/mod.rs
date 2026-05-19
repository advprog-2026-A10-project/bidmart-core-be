use thiserror::Error;

#[derive(Error, Debug)]
pub enum BiddingError {
    #[error("Auction not found")]
    AuctionNotFound,

    #[error("Bid not found")]
    BidNotFound,

    #[error("Auction is not active")]
    AuctionNotActive,

    #[error("Auction has ended")]
    AuctionEnded,

    #[error("Bid amount is too low")]
    BidTooLow,

    #[error("Insufficient available balance")]
    InsufficientBalance,

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Internal error: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for BiddingError {
    fn into_response(self) -> axum::response::Response {
        match self {
            BiddingError::ValidationError(message) => {
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
                    BiddingError::AuctionNotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Auction not found".to_string(),
                    ),
                    BiddingError::BidNotFound => (
                        axum::http::StatusCode::NOT_FOUND,
                        "Bid not found".to_string(),
                    ),
                    BiddingError::AuctionNotActive => (
                        axum::http::StatusCode::CONFLICT,
                        "Auction is not active".to_string(),
                    ),
                    BiddingError::AuctionEnded => (
                        axum::http::StatusCode::CONFLICT,
                        "Auction has ended".to_string(),
                    ),
                    BiddingError::BidTooLow => (
                        axum::http::StatusCode::CONFLICT,
                        "Bid amount is too low".to_string(),
                    ),
                    BiddingError::InsufficientBalance => (
                        axum::http::StatusCode::CONFLICT,
                        "Insufficient available balance".to_string(),
                    ),
                    BiddingError::Unauthorized => (
                        axum::http::StatusCode::UNAUTHORIZED,
                        "Unauthorized".to_string(),
                    ),
                    BiddingError::DatabaseError(_) | BiddingError::InternalError(_) => (
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                        "Internal server error".to_string(),
                    ),
                    BiddingError::ValidationError(_) => unreachable!(),
                };
                let body = axum::Json(serde_json::json!({ "message": message }));
                (status, body).into_response()
            }
        }
    }
}
