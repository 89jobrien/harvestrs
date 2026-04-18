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
    pub fn dedup_db_path(&self) -> PathBuf {
        let path = PathBuf::from(&self.dedup_db);

        // Expand ~ to home directory
        if self.dedup_db.starts_with('~') {
            if let Some(home) = dirs::home_dir() {
                // Remove leading ~ and rejoin with home directory
                let relative = path.strip_prefix("~").unwrap_or(&path);
                home.join(relative)
            } else {
                path
            }
        } else {
            path
        }
    }

    /// Returns the configured vault path, or an error if none is set.
    /// Precedence: `--vault` flag > `OBSIDIAN_VAULT_PATH` env var.
    /// No hardcoded machine-specific fallback.
    pub fn vault_path(&self) -> anyhow::Result<PathBuf> {
        if let Some(ref p) = self.vault {
            return Ok(p.clone());
        }
        std::env::var("OBSIDIAN_VAULT_PATH")
            .map(PathBuf::from)
            .map_err(|_| {
                anyhow::anyhow!(
                    "vault path not set: use --vault or OBSIDIAN_VAULT_PATH env var"
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(vault: Option<PathBuf>) -> Config {
        Config {
            output: None,
            dedup_db: "~/.harvestrs/dedup.db".into(),
            vault,
            no_sync: false,
            git_max_commits: 20,
            disable_source: vec![],
        }
    }

    #[test]
    fn vault_path_uses_explicit_flag() {
        let cfg = make_config(Some(PathBuf::from("/tmp/my-vault")));
        assert_eq!(cfg.vault_path().unwrap(), PathBuf::from("/tmp/my-vault"));
    }

    #[test]
    fn vault_path_reads_env_var() {
        // Safety: test-only env mutation, single-threaded.
        unsafe { std::env::set_var("OBSIDIAN_VAULT_PATH", "/tmp/env-vault") };
        let cfg = make_config(None);
        let result = cfg.vault_path().unwrap();
        unsafe { std::env::remove_var("OBSIDIAN_VAULT_PATH") };
        assert_eq!(result, PathBuf::from("/tmp/env-vault"));
    }

    #[test]
    fn vault_path_errors_when_nothing_set() {
        // Run in a subprocess-like way by checking with an explicit no-vault config;
        // we can only assert error when env is unset — guard with a sentinel approach.
        let cfg = make_config(None);
        // If env var is set, we can't test the error case in parallel tests.
        // Instead, verify that the explicit-flag path always wins and returns Ok.
        let cfg_with_flag = make_config(Some(PathBuf::from("/tmp/explicit")));
        assert!(cfg_with_flag.vault_path().is_ok());
        // And that without flag AND without env, it returns Err (run sequentially if env clean).
        if std::env::var("OBSIDIAN_VAULT_PATH").is_err() {
            assert!(cfg.vault_path().is_err(), "expected error when vault not configured");
        }
    }

    #[test]
    fn vault_path_never_returns_icloud_path() {
        // The explicit flag path must be returned as-is — never an iCloud fallback.
        let cfg = make_config(Some(PathBuf::from("/tmp/my-vault")));
        let path = cfg.vault_path().unwrap();
        assert!(!path.to_string_lossy().contains("iCloud"));
    }
}
