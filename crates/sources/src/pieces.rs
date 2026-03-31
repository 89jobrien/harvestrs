use async_trait::async_trait;
use harvester_core::{HarvestError, HarvestedItem, Source, SourceId};

pub struct PiecesSource {
    base_url: String,
    max_items: usize,
}

impl PiecesSource {
    pub fn new(base_url: String, max_items: usize) -> Self {
        Self {
            base_url,
            max_items,
        }
    }

    pub fn new_default() -> Self {
        Self::new("http://localhost:39300".into(), 50)
    }

    async fn is_available(&self) -> bool {
        reqwest::get(format!("{}/.well-known/health", self.base_url))
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[async_trait]
impl Source for PiecesSource {
    fn id(&self) -> &str {
        "pieces"
    }

    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError> {
        if !self.is_available().await {
            return Ok(vec![]);
        }

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{}/activities", self.base_url))
            .query(&[("limit", self.max_items.to_string())])
            .send()
            .await
            .map_err(|e| HarvestError::SourceFailed {
                source_id: "pieces".into(),
                reason: e.to_string(),
            })?;

        if !resp.status().is_success() {
            return Ok(vec![]);
        }

        let json: serde_json::Value =
            resp.json().await.map_err(|e| HarvestError::SourceFailed {
                source_id: "pieces".into(),
                reason: e.to_string(),
            })?;

        let mut items = Vec::new();
        if let Some(activities) = json.as_array() {
            for activity in activities.iter().take(self.max_items) {
                let content = activity
                    .get("description")
                    .or_else(|| activity.get("summary"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if content.is_empty() {
                    continue;
                }
                items.push(HarvestedItem::new(
                    SourceId("pieces".into()),
                    content,
                    serde_json::json!({ "raw": activity }),
                ));
            }
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harvester_core::Source;

    #[tokio::test]
    async fn returns_empty_when_pieces_unavailable() {
        let source = PiecesSource::new("http://localhost:39301".into(), 5);
        let items = source.harvest().await.unwrap();
        assert_eq!(items.len(), 0);
    }
}
