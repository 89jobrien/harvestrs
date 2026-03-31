use async_trait::async_trait;
use harvester_core::{HarvestError, HarvestedItem, Source, SourceId};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct ObsidianSource {
    daily_dir: PathBuf,
}

impl ObsidianSource {
    pub fn new(daily_dir: PathBuf) -> Self {
        Self { daily_dir }
    }

    pub fn default_path() -> PathBuf {
        std::env::var("OBSIDIAN_VAULT_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_default()
                    .join("Library/Mobile Documents/iCloud~md~obsidian/Documents/air-vault")
            })
            .join("_daily")
    }
}

#[async_trait]
impl Source for ObsidianSource {
    fn id(&self) -> &str {
        "obsidian"
    }

    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError> {
        let mut items = Vec::new();
        for entry in WalkDir::new(&self.daily_dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |x| x == "md"))
        {
            let content = std::fs::read_to_string(entry.path())
                .map_err(HarvestError::Io)?
                .trim()
                .to_string();
            if content.is_empty() {
                continue;
            }
            let date = entry
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            items.push(HarvestedItem::new(
                SourceId("obsidian".into()),
                content,
                serde_json::json!({ "date": date }),
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
    async fn harvests_daily_note_content() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("2026-03-30.md"),
            "# 2026-03-30\n\nFixed rl_prompt test failures.",
        )
        .unwrap();

        let source = ObsidianSource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("Fixed rl_prompt test failures."));
        assert_eq!(items[0].source.0, "obsidian");
    }

    #[tokio::test]
    async fn skips_empty_notes() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("2026-03-29.md"), "").unwrap();

        let source = ObsidianSource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();
        assert_eq!(items.len(), 0);
    }
}
