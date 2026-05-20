use async_trait::async_trait;
use uuid::Uuid;

use crate::modules::catalog::domain::entities::Listing;
use crate::modules::catalog::domain::errors::ListingError;

/// Outbound port the catalog uses to start an auction at publish time.
///
/// Implementations are responsible for inserting the auctions row and linking
/// `listings.auction_id` atomically. The catalog itself never owns auction
/// schema knowledge beyond invoking this port.
#[async_trait]
pub trait AuctionLifecyclePort: Send + Sync {
    /// Create an auction row for the given (already published) listing and
    /// link it back via `listings.auction_id`. Returns the new auction id.
    ///
    /// Implementations must be idempotent against partial completion: if a
    /// listing already has `auction_id` set, callers should not invoke this.
    async fn start_auction_for_listing(
        &self,
        listing: &Listing,
        image_url: Option<String>,
    ) -> Result<Uuid, ListingError>;
}
