use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "harvestrs",
    about = "Sync memory + harvest data sources -> JSONL"
)]
pub struct Config {
    /// Output file for harvested items (default: stdout)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// SQLite dedup database path
    #[arg(long, default_value = "~/.harvestrs/dedup.db")]
    pub dedup_db: String,

    /// Obsidian vault root (overrides OBSIDIAN_VAULT_PATH env var)
    #[arg(long)]
    pub vault: Option<PathBuf>,

    /// Skip memory sync to vault
    #[arg(long, default_value_t = false)]
    pub no_sync: bool,

    /// Max git commits per repo to harvest
    #[arg(long, default_value_t = 20)]
    pub git_max_commits: usize,

    /// Disable a source (can specify multiple): facets, memory, git, obsidian, pieces
    #[arg(long)]
    pub disable_source: Vec<String>,
}

impl Config {
    pub fn dedup_db_path(&self) -> String {
        if self.dedup_db.starts_with('~') {
            self.dedup_db.replacen(
                '~',
                &dirs::home_dir().unwrap_or_default().to_string_lossy(),
                1,
            )
        } else {
            self.dedup_db.clone()
        }
    }

    pub fn vault_path(&self) -> PathBuf {
        self.vault.clone().unwrap_or_else(|| {
            std::env::var("OBSIDIAN_VAULT_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::home_dir()
                        .unwrap_or_default()
                        .join("Library/Mobile Documents/iCloud~md~obsidian/Documents/air-vault")
                })
        })
    }
}
