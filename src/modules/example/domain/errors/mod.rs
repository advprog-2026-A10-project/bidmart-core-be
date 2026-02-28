use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum CatalogError {
    // Category errors
    #[error("Category not found")]
    CategoryNotFound,
    #[error("Category already exists")]
    CategoryAlreadyExists,

    // Listing errors
    #[error("Listing not found")]
    ListingNotFound,
    #[error("Listing not active")]
    ListingNotActive,
    #[error("Auction has ended")]
    AuctionEnded,
    #[error("Bid amount too low")]
    BidTooLow,

    // Bid errors
    #[error("Bid not found")]
    BidNotFound,

    // Wallet errors
    #[error("Wallet not found")]
    WalletNotFound,
    #[error("Insufficient balance")]
    InsufficientBalance,

    // Order errors
    #[error("Order not found")]
    OrderNotFound,
    #[error("Invalid shipping status transition")]
    InvalidShippingStatusTransition,

    // Notification errors
    #[error("Notification not found")]
    NotificationNotFound,

    // Generic errors
    #[error("Invalid data")]
    InvalidData,
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("External service error: {0}")]
    ExternalServiceError(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Not found: {0}")]
    NotFound(String),
}

impl serde::Serialize for CatalogError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
