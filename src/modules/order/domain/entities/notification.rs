use super::order::OrderId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: NotificationId,
    pub order_id: Option<OrderId>,
    pub title: String,
    pub body: String,
    pub channel: NotificationChannel,
    pub notification_type: NotificationType,
    pub created_at: String,
    pub read_at: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

pub type NotificationId = Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannel {
    Email,
    Push,
    Inbox,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NotificationType {
    #[serde(rename = "BID_OUTBID")]
    BidOutbid,
    #[serde(rename = "AUCTION_WON")]
    AuctionWon,
    #[serde(rename = "AUCTION_LOST")]
    AuctionLost,
    #[serde(rename = "ORDER_SHIPPED")]
    OrderShipped,
    #[serde(rename = "ORDER_DELIVERED")]
    OrderDelivered,
    #[serde(rename = "PAYMENT_RECEIVED")]
    PaymentReceived,
    #[serde(rename = "DISPUTE_OPENED")]
    DisputeOpened,
    #[serde(rename = "DISPUTE_RESOLVED")]
    DisputeResolved,
    #[serde(rename = "AUCTION_EXTENDED")]
    AuctionExtended,
    #[serde(rename = "BidPlaced")]
    BidPlaced,
    #[serde(rename = "WinnerDetermined")]
    WinnerDetermined,
    #[serde(rename = "OrderUpdate")]
    OrderUpdate,
    #[serde(rename = "System")]
    System,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEventPayload {
    pub order_id: Option<OrderId>,
    #[serde(rename = "type")]
    pub notification_type: NotificationType,
    pub title: String,
    pub body: String,
    pub channel: NotificationChannel,
    pub metadata: Option<serde_json::Value>,
}

impl NotificationType {
    pub fn from_db_value(value: &str) -> Option<Self> {
        match value {
            "BID_OUTBID" => Some(Self::BidOutbid),
            "AUCTION_WON" => Some(Self::AuctionWon),
            "AUCTION_LOST" => Some(Self::AuctionLost),
            "ORDER_SHIPPED" => Some(Self::OrderShipped),
            "ORDER_DELIVERED" => Some(Self::OrderDelivered),
            "PAYMENT_RECEIVED" => Some(Self::PaymentReceived),
            "DISPUTE_OPENED" => Some(Self::DisputeOpened),
            "DISPUTE_RESOLVED" => Some(Self::DisputeResolved),
            "AUCTION_EXTENDED" => Some(Self::AuctionExtended),
            _ => None,
        }
    }

    pub fn as_db_value(&self) -> &'static str {
        match self {
            Self::BidOutbid | Self::BidPlaced => "BID_OUTBID",
            Self::AuctionWon | Self::WinnerDetermined => "AUCTION_WON",
            Self::AuctionLost => "AUCTION_LOST",
            Self::OrderShipped | Self::OrderUpdate => "ORDER_SHIPPED",
            Self::OrderDelivered => "ORDER_DELIVERED",
            Self::PaymentReceived => "PAYMENT_RECEIVED",
            Self::DisputeOpened => "DISPUTE_OPENED",
            Self::DisputeResolved => "DISPUTE_RESOLVED",
            Self::AuctionExtended | Self::System => "AUCTION_EXTENDED",
        }
    }
}
