use thiserror::Error;

#[derive(Error, Debug)]
pub enum OrderError {
    #[error("Order not found")]
    NotFound,
    #[error("Invalid order transition")]
    InvalidTransition,
    #[error("Database error")]
    Database(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Notification not found")]
    NotFound,
    #[error("Notification already marked as read")]
    AlreadyRead,
    #[error("Database error")]
    Database(#[from] anyhow::Error),
}
