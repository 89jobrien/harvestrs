use anyhow::Result;
use rusqlite::Connection;

/// SQLite-backed store tracking which ContentHashes have been emitted.
#[allow(dead_code)]
pub struct DedupStore {
    conn: Connection,
}

impl DedupStore {
    #[allow(dead_code)]
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS seen_hashes (
                hash TEXT PRIMARY KEY,
                seen_at TEXT NOT NULL
            );",
        )?;
        Ok(Self { conn })
    }

    #[allow(dead_code)]
    pub fn seen(&self, hash: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM seen_hashes WHERE hash = ?1",
            [hash],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    #[allow(dead_code)]
    pub fn mark_seen(&self, hash: &str) -> Result<()> {
        self.conn.execute(
            "INSERT OR IGNORE INTO seen_hashes (hash, seen_at) VALUES (?1, datetime('now'))",
            [hash],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_hash_is_unseen() {
        let store = DedupStore::open(":memory:").unwrap();
        assert!(!store.seen("abc123").unwrap());
    }

    #[test]
    fn mark_seen_prevents_duplicates() {
        let store = DedupStore::open(":memory:").unwrap();
        store.mark_seen("abc123").unwrap();
        assert!(store.seen("abc123").unwrap());
    }

    #[test]
    fn different_hashes_are_independent() {
        let store = DedupStore::open(":memory:").unwrap();
        store.mark_seen("aaa").unwrap();
        assert!(!store.seen("bbb").unwrap());
    }
}
