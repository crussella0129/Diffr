use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::drive::DriveId;

/// A file or directory entry in the index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    /// Relative path from the drive root.
    pub rel_path: PathBuf,
    /// Which drive this entry belongs to.
    pub drive_id: DriveId,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modification time from the filesystem.
    pub mtime: DateTime<Utc>,
    /// XXH3-64 hash for fast change detection (hex string).
    pub xxh3_hash: Option<String>,
    /// SHA-256 hash for verification (hex string).
    pub sha256_hash: Option<String>,
    /// When this entry was last indexed.
    pub indexed_at: DateTime<Utc>,
}

/// Cached hash entry for avoiding re-hashing unchanged files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashCacheEntry {
    /// Relative path from the drive root.
    pub rel_path: PathBuf,
    pub drive_id: DriveId,
    /// File size at time of hashing.
    pub size: u64,
    /// Modification time at time of hashing.
    pub mtime: DateTime<Utc>,
    /// XXH3-64 hash (hex string).
    pub xxh3_hash: String,
    /// SHA-256 hash if computed (hex string).
    pub sha256_hash: Option<String>,
    /// When this cache entry was created.
    pub cached_at: DateTime<Utc>,
}

impl HashCacheEntry {
    /// Check if this cache entry is still valid for the given file metadata.
    pub fn is_valid(&self, size: u64, mtime: DateTime<Utc>) -> bool {
        self.size == size && self.mtime == mtime
    }
}
