use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::drive::DriveId;

/// A version of a file stored in the archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    pub id: Uuid,
    /// The original relative path of the file.
    pub original_path: PathBuf,
    /// Where the archived version is stored.
    pub archive_path: PathBuf,
    /// Which drive this archive is stored on.
    pub drive_id: DriveId,
    /// Original file size before compression.
    pub original_size: u64,
    /// Compressed size on disk.
    pub compressed_size: u64,
    /// Compression format used.
    pub compression: CompressionFormat,
    /// XXH3 hash of the original file.
    pub xxh3_hash: String,
    /// Why this file was archived.
    pub reason: ArchiveReason,
    /// When this version was archived.
    pub archived_at: DateTime<Utc>,
}

/// Why a file was archived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveReason {
    /// Archived before being overwritten by a newer version.
    BeforeOverwrite,
    /// Archived before being deleted.
    BeforeDelete,
    /// Manual archive by user.
    Manual,
}

impl std::fmt::Display for ArchiveReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArchiveReason::BeforeOverwrite => write!(f, "before_overwrite"),
            ArchiveReason::BeforeDelete => write!(f, "before_delete"),
            ArchiveReason::Manual => write!(f, "manual"),
        }
    }
}

/// Compression format for archived files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompressionFormat {
    None,
    Zstd,
}

impl std::fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionFormat::None => write!(f, "none"),
            CompressionFormat::Zstd => write!(f, "zstd"),
        }
    }
}

/// Policy governing archive retention.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Maximum age of archived versions in days. None = keep forever.
    pub max_age_days: Option<u32>,
    /// Maximum number of versions to keep per file. None = unlimited.
    pub max_versions: Option<u32>,
    /// Maximum total archive size in bytes. None = unlimited.
    pub max_total_bytes: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_age_days: Some(90),
            max_versions: Some(10),
            max_total_bytes: None,
        }
    }
}
