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
        // Include the source ID in the hash so that identical content from different
        // sources is not incorrectly deduplicated. Content alone is not a sufficient
        // uniqueness key across heterogeneous sources.
        let mut hasher = Sha256::new();
        hasher.update(source.0.as_bytes());
        hasher.update(b"\x00");
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());
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
    fn same_content_different_source_produces_different_id() {
        let item_a = HarvestedItem::new(
            SourceId("facets".into()),
            "shared content".into(),
            serde_json::Value::Null,
        );
        let item_b = HarvestedItem::new(
            SourceId("obsidian".into()),
            "shared content".into(),
            serde_json::Value::Null,
        );
        assert_ne!(
            item_a.id, item_b.id,
            "items from different sources must not share a content hash"
        );
    }

    #[test]
    fn same_content_same_source_produces_same_id() {
        let item_a = HarvestedItem::new(
            SourceId("facets".into()),
            "stable content".into(),
            serde_json::Value::Null,
        );
        let item_b = HarvestedItem::new(
            SourceId("facets".into()),
            "stable content".into(),
            serde_json::json!({"different": "metadata"}),
        );
        assert_eq!(
            item_a.id, item_b.id,
            "same source + same content must produce the same hash regardless of metadata"
        );
    }

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
