use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Opaque content hash used for deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(pub String);

/// Identifies which source produced an item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceId(pub String);

/// A single harvested unit of data from any source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestedItem {
    /// SHA-256 of `content` — used for dedup.
    pub id: ContentHash,
    /// Which adapter produced this item.
    pub source: SourceId,
    /// Raw text content (session summary, commit message, note body, etc.)
    pub content: String,
    /// Source-specific structured metadata (file path, repo, timestamp, etc.)
    pub metadata: serde_json::Value,
    pub harvested_at: DateTime<Utc>,
}

impl HarvestedItem {
    pub fn new(source: SourceId, content: String, metadata: serde_json::Value) -> Self {
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(content.as_bytes()));
        Self {
            id: ContentHash(hash),
            source,
            content,
            metadata,
            harvested_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harvested_item_round_trips_json() {
        let item = HarvestedItem {
            id: ContentHash("abc123".into()),
            source: SourceId("facets".into()),
            content: "test content".into(),
            metadata: serde_json::json!({"key": "value"}),
            harvested_at: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&item).unwrap();
        let decoded: HarvestedItem = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id.0, "abc123");
        assert_eq!(decoded.source.0, "facets");
        assert_eq!(decoded.content, "test content");
    }
}
