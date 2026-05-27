use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ListingImage {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub url: String,
    pub order: i32,
    pub created_at: DateTime<Utc>,
}
