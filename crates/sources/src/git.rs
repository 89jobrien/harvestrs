use async_trait::async_trait;
use harvester_core::{HarvestError, HarvestedItem, Source, SourceId};
use std::path::PathBuf;

pub struct GitSource {
    repo_dirs: Vec<PathBuf>,
    max_commits_per_repo: usize,
}

impl GitSource {
    pub fn new(repo_dirs: Vec<PathBuf>, max_commits_per_repo: usize) -> Self {
        Self {
            repo_dirs,
            max_commits_per_repo,
        }
    }

    pub fn default_repos() -> Vec<PathBuf> {
        let dev = dirs::home_dir().unwrap_or_default().join("dev");
        std::fs::read_dir(&dev)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().join(".git").exists())
                    .map(|e| e.path())
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[async_trait]
impl Source for GitSource {
    fn id(&self) -> &str {
        "git"
    }

    async fn harvest(&self) -> Result<Vec<HarvestedItem>, HarvestError> {
        let mut items = Vec::new();
        for repo_path in &self.repo_dirs {
            let repo = match git2::Repository::open(repo_path) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let mut revwalk = repo.revwalk().map_err(|e| HarvestError::SourceFailed {
                source_id: "git".into(),
                reason: e.to_string(),
            })?;
            revwalk.push_head().ok();
            revwalk.set_sorting(git2::Sort::TIME).ok();

            let repo_name = repo_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            for (i, oid) in revwalk.enumerate() {
                if i >= self.max_commits_per_repo {
                    break;
                }
                let oid = match oid {
                    Ok(o) => o,
                    Err(_) => continue,
                };
                let commit = match repo.find_commit(oid) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let message = commit.message().unwrap_or("").trim().to_string();
                if message.is_empty() {
                    continue;
                }
                let author = commit.author().name().unwrap_or("unknown").to_string();
                let time = commit.time().seconds();

                items.push(HarvestedItem::new(
                    SourceId("git".into()),
                    format!("[{}] {}", repo_name, message),
                    serde_json::json!({
                        "repo": repo_name,
                        "commit": oid.to_string(),
                        "author": author,
                        "timestamp": time,
                    }),
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
    use std::process::Command;
    use tempfile::TempDir;

    fn make_repo_with_commit(dir: &std::path::Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
        std::fs::write(dir.join("README.md"), "hello").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feat: initial commit"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[tokio::test]
    async fn harvests_commits_from_repo() {
        let dir = TempDir::new().unwrap();
        make_repo_with_commit(dir.path());

        let source = GitSource::new(vec![dir.path().to_path_buf()], 10);
        let items = source.harvest().await.unwrap();

        assert_eq!(items.len(), 1);
        assert!(items[0].content.contains("feat: initial commit"));
        assert_eq!(items[0].source.0, "git");
    }

    #[tokio::test]
    async fn non_repo_dir_is_skipped_gracefully() {
        let dir = TempDir::new().unwrap();
        let source = GitSource::new(vec![dir.path().to_path_buf()], 10);
        let items = source.harvest().await.unwrap();
        assert_eq!(items.len(), 0);
    }
}
