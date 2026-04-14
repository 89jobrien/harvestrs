# harvestrs

Polyglot data harvester — syncs Claude memory, git commits, Obsidian notes, and Pieces activities to JSONL.

## Install

```bash
cargo install --path crates/cli
```

## Usage

```
harvestrs [OPTIONS]
```

### Options

```
-o, --output <FILE>              Write harvested items to a file (default: stdout)
    --dedup-db <PATH>            SQLite dedup database (default: ~/.harvestrs/dedup.db)
    --vault <PATH>               Obsidian vault root (overrides OBSIDIAN_VAULT_PATH env var)
    --no-sync                    Skip syncing Claude memory to vault
    --git-max-commits <N>        Max git commits to harvest per repo (default: 20)
    --disable-source <SOURCE>    Disable a source; can repeat. Values: facets, memory, git, obsidian, pieces
```

### Sources

| Source | What it collects |
|--------|-----------------|
| `memory` | Claude Code project memory from `~/.claude/projects` |
| `facets` | Claude usage-data facets from `~/.claude/usage-data/facets` |
| `git` | Recent commits from git repos detected under `~/dev` |
| `obsidian` | Daily notes from `<vault>/_daily` |
| `pieces` | Pieces app activity via its local API |

Each run syncs new Claude memory files to the Obsidian vault, then harvests all enabled sources, deduplicates against the SQLite store, and emits new items as JSONL to stdout or a file.

### Examples

```bash
# Harvest all sources to stdout
harvestrs

# Write to a file, skipping git
harvestrs -o out.jsonl --disable-source git

# Use a custom vault path
harvestrs --vault ~/Documents/my-vault

# Disable memory sync and limit git commits
harvestrs --no-sync --git-max-commits 5
```

## License

MIT OR Apache-2.0
