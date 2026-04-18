use async_trait::async_trait;
use harvester_core::{HarvestError, HarvestedItem, Source, SourceId};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct FacetsSource {
    facets_dir: PathBuf,
}

impl FacetsSource {
    pub fn new(facets_dir: PathBuf) -> Self {
        Self { facets_dir }
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/usage-data/facets")
    }
}

#[async_trait]
impl Source for FacetsSource {
    fn id(&self) -> &str {
        "facets"
    }

    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError> {
        let mut items = Vec::new();
        for entry in WalkDir::new(&self.facets_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
        {
            let raw = match std::fs::read_to_string(entry.path()) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!(
                        "[harvestrs/facets] failed to read {}: {e}",
                        entry.path().display()
                    );
                    continue;
                }
            };
            let json: serde_json::Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!(
                        "[harvestrs/facets] skipping malformed JSON at {}: {e}",
                        entry.path().display()
                    );
                    continue;
                }
            };

            let content = [
                json.get("underlying_goal")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                json.get("brief_summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
                json.get("friction_detail")
                    .and_then(|v| v.as_str())
                    .unwrap_or(""),
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

            if content.is_empty() {
                continue;
            }

            items.push(HarvestedItem::new(
                SourceId("facets".into()),
                content,
                serde_json::json!({
                    "file": entry.path().file_name().unwrap().to_string_lossy(),
                    "outcome": json.get("outcome"),
                    "session_id": json.get("session_id"),
                }),
            ));
        }
        Ok(items)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use harvester_core::Source;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn harvests_facet_json_files() {
        let dir = TempDir::new().unwrap();
        let facet = serde_json::json!({
            "session_id": "abc-123",
            "brief_summary": "User fixed a bug in rl_prompt",
            "outcome": "fully_achieved",
            "underlying_goal": "fix test failures"
        });
        fs::write(dir.path().join("abc-123.json"), facet.to_string()).unwrap();

        let source = FacetsSource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source.0, "facets");
        assert!(items[0].content.contains("fix test failures"));
    }

    #[tokio::test]
    async fn skips_malformed_json_and_continues() {
        let dir = TempDir::new().unwrap();
        // Write one malformed and one valid facet file.
        fs::write(dir.path().join("bad.json"), "{ not valid json !!!").unwrap();
        let facet = serde_json::json!({
            "session_id": "good-123",
            "brief_summary": "Valid item",
            "outcome": "fully_achieved",
            "underlying_goal": "do something"
        });
        fs::write(dir.path().join("good.json"), facet.to_string()).unwrap();

        let source = FacetsSource::new(dir.path().to_path_buf());
        // Must not return Err — malformed file is skipped, valid file is harvested.
        let items = source.harvest().await.expect("harvest should not abort on bad JSON");
        assert_eq!(items.len(), 1, "expected 1 valid item, got {}", items.len());
        assert!(items[0].content.contains("Valid item"));
    }

    #[tokio::test]
    async fn skips_non_json_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("notes.txt"), "not a facet").unwrap();

        let source = FacetsSource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();
        assert_eq!(items.len(), 0);
    }
}
