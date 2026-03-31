mod config;
mod dedup;
mod sync;

use anyhow::Result;
use clap::Parser;
use config::Config;
use dedup::DedupStore;
use harvester_core::Source;
use sources::{FacetsSource, GitSource, MemorySource, ObsidianSource, PiecesSource};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::parse();

    // 1. Sync memory to vault
    if !cfg.no_sync {
        let projects_root = dirs::home_dir()
            .unwrap_or_default()
            .join(".claude/projects");
        let vault_root = cfg.vault_path();
        if vault_root.exists() {
            sync::sync_all(&projects_root, &vault_root)?;
            eprintln!("[harvestrs] memory synced to vault");
        } else {
            eprintln!(
                "[harvestrs] vault not found at {}, skipping sync",
                vault_root.display()
            );
        }
    }

    // 2. Build enabled sources
    let disabled = &cfg.disable_source;
    let all_sources: Vec<Box<dyn Source>> = vec![
        Box::new(FacetsSource::new(
            dirs::home_dir()
                .unwrap_or_default()
                .join(".claude/usage-data/facets"),
        )),
        Box::new(MemorySource::new(
            dirs::home_dir().unwrap_or_default().join(".claude/projects"),
        )),
        Box::new(GitSource::new(
            GitSource::default_repos(),
            cfg.git_max_commits,
        )),
        Box::new(ObsidianSource::new(cfg.vault_path().join("_daily"))),
        Box::new(PiecesSource::default()),
    ];
    let sources: Vec<Box<dyn Source>> = all_sources
        .into_iter()
        .filter(|s| !disabled.contains(&s.id().to_string()))
        .collect();

    // 3. Open dedup store
    let db_path = cfg.dedup_db_path();
    if let Some(parent) = std::path::Path::new(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let dedup = DedupStore::open(&db_path)?;

    // 4. Harvest all sources, deduplicate, write JSONL
    let mut writer: Box<dyn Write> = match &cfg.output {
        Some(path) => Box::new(std::fs::File::create(path)?),
        None => Box::new(std::io::stdout()),
    };

    let mut total = 0usize;
    let mut new_count = 0usize;
    for source in &sources {
        match source.harvest().await {
            Ok(items) => {
                for item in items {
                    total += 1;
                    if dedup.seen(&item.id.0)? {
                        continue;
                    }
                    dedup.mark_seen(&item.id.0)?;
                    writeln!(writer, "{}", serde_json::to_string(&item)?)?;
                    new_count += 1;
                }
            }
            Err(e) => eprintln!("[harvestrs] source '{}' error: {}", source.id(), e),
        }
    }

    eprintln!(
        "[harvestrs] harvested {} new items ({} total, {} deduped)",
        new_count,
        total,
        total - new_count
    );
    Ok(())
}
