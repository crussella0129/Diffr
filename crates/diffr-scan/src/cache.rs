use chrono::{DateTime, Utc};
use diffr_core::models::drive::DriveId;
use diffr_core::models::file_entry::HashCacheEntry;
use diffr_db::ops;
use rusqlite::Connection;
use std::path::Path;

use crate::hasher;

/// Hash cache backed by SQLite. Avoids re-hashing files that haven't changed.
pub struct HashCache<'a> {
    conn: &'a Connection,
    drive_id: DriveId,
}

impl<'a> HashCache<'a> {
    pub fn new(conn: &'a Connection, drive_id: DriveId) -> Self {
        Self { conn, drive_id }
    }

    /// Get or compute the hash for a file. Uses cache if (size, mtime) match.
    pub fn get_or_hash(
        &self,
        root: &Path,
        rel_path: &Path,
        size: u64,
        mtime: DateTime<Utc>,
        include_sha256: bool,
    ) -> anyhow::Result<hasher::HashResult> {
        let rel_str = rel_path.to_string_lossy();

        // Check cache
        if let Some(cached) =
            ops::get_hash_cache_entry(self.conn, &self.drive_id, &rel_str)?
        {
            if cached.is_valid(size, mtime) {
                return Ok(hasher::HashResult {
                    xxh3_hex: cached.xxh3_hash,
                    sha256_hex: cached.sha256_hash,
                });
            }
        }

        // Cache miss — compute hash
        let full_path = root.join(rel_path);
        let result = hasher::hash_file(&full_path, include_sha256)?;

        // Store in cache
        let cache_entry = HashCacheEntry {
            rel_path: rel_path.to_path_buf(),
            drive_id: self.drive_id.clone(),
            size,
            mtime,
            xxh3_hash: result.xxh3_hex.clone(),
            sha256_hash: result.sha256_hex.clone(),
            cached_at: Utc::now(),
        };
        ops::upsert_hash_cache(self.conn, &cache_entry)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diffr_core::models::drive::{Drive, DriveIdentity};
    use tempfile::TempDir;

    #[test]
    fn test_hash_cache_hit() {
        let conn = diffr_db::open_memory_db().unwrap();
        // Insert a drive so FK constraint is satisfied
        let drive = Drive::new(DriveIdentity::new_synthetic(), "/tmp/test".into());
        let drive_id = drive.id.clone();
        diffr_db::ops::insert_drive(&conn, &drive).unwrap();
        let cache = HashCache::new(&conn, drive_id.clone());

        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let mtime = Utc::now();

        // First call: cache miss
        let r1 = cache
            .get_or_hash(dir.path(), Path::new("test.txt"), 5, mtime, false)
            .unwrap();

        // Second call: cache hit (same size and mtime)
        let r2 = cache
            .get_or_hash(dir.path(), Path::new("test.txt"), 5, mtime, false)
            .unwrap();

        assert_eq!(r1.xxh3_hex, r2.xxh3_hex);
    }
}
