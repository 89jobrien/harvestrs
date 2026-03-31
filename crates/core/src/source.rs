use crate::{HarvestError, HarvestedItem};
use async_trait::async_trait;

/// Port: any external data source that can produce HarvestedItems.
/// Adapters in `sources` implement this trait.
#[async_trait]
pub trait Source: Send + Sync {
    /// Unique identifier for this source (used in SourceId and logs).
    fn id(&self) -> &str;

    /// Harvest new items. Implementations should return all available items;
    /// deduplication is handled by the caller via ContentHash.
    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError>;
}
