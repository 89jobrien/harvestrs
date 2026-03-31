use async_trait::async_trait;
use harvester_core::{HarvestError, HarvestedItem, Source, SourceId};
use std::path::PathBuf;
use walkdir::WalkDir;

pub struct MemorySource {
    projects_root: PathBuf,
}

impl MemorySource {
    pub fn new(projects_root: PathBuf) -> Self {
        Self { projects_root }
    }

    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/projects")
    }
}

#[async_trait]
impl Source for MemorySource {
    fn id(&self) -> &str {
        "memory"
    }

    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError> {
        let mut items = Vec::new();
        for entry in WalkDir::new(&self.projects_root)
            .min_depth(3)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().is_some_and(|x| x == "md")
                    && e.path().file_name().is_some_and(|n| n != "MEMORY.md")
                    && e.path()
                        .parent()
                        .and_then(|p| p.file_name())
                        .is_some_and(|n| n == "memory")
            })
        {
            let content = std::fs::read_to_string(entry.path()).map_err(HarvestError::Io)?;
            let body = if content.starts_with("---\n") {
                content
                    .splitn(3, "---\n")
                    .nth(2)
                    .unwrap_or(&content)
                    .trim()
                    .to_string()
            } else {
                content.trim().to_string()
            };

            if body.is_empty() {
                continue;
            }

            let project_slug = entry
                .path()
                .ancestors()
                .nth(2)
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            items.push(HarvestedItem::new(
                SourceId("memory".into()),
                body,
                serde_json::json!({
                    "file": entry.path().file_name().unwrap().to_string_lossy(),
                    "project": project_slug,
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
    async fn harvests_memory_md_files() {
        let dir = TempDir::new().unwrap();
        let proj = dir.path().join("-Users-joe-dev-minibox").join("memory");
        fs::create_dir_all(&proj).unwrap();
        fs::write(
            proj.join("project_state.md"),
            "---\nname: state\ntype: project\n---\nAll tests passing.",
        )
        .unwrap();
        fs::write(proj.join("MEMORY.md"), "# index").unwrap();

        let source = MemorySource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("All tests passing."));
        assert_eq!(items[0].source.0, "memory");
    }

    #[tokio::test]
    async fn skips_memory_md_index() {
        let dir = TempDir::new().unwrap();
        let proj = dir.path().join("proj").join("memory");
        fs::create_dir_all(&proj).unwrap();
        fs::write(proj.join("MEMORY.md"), "# index").unwrap();

        let source = MemorySource::new(dir.path().to_path_buf());
        let items = source.harvest().await.unwrap();
        assert_eq!(items.len(), 0);
    }
}
