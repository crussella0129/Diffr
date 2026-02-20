use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

use super::cluster::ClusterId;

/// Unique identifier for a drive within Diffr.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DriveId(pub Uuid);

impl DriveId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for DriveId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How a drive was identified.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DriveIdentity {
    /// Identified by hardware serial number.
    Hardware { serial: String },
    /// Synthetic identity stored on the drive itself.
    Synthetic { id: String },
}

impl DriveIdentity {
    pub fn new_hardware(serial: String) -> Self {
        DriveIdentity::Hardware { serial }
    }

    pub fn new_synthetic() -> Self {
        DriveIdentity::Synthetic {
            id: Uuid::new_v4().to_string(),
        }
    }

    pub fn identity_string(&self) -> &str {
        match self {
            DriveIdentity::Hardware { serial } => serial,
            DriveIdentity::Synthetic { id } => id,
        }
    }
}

/// Role of a drive within a cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriveRole {
    /// Normal sync participant — syncs files and may store archives.
    Normal,
    /// Assists with archiving — stores extra archive copies.
    ArchiveAssist,
    /// Archive-only — does not participate in active sync, only stores archives.
    ArchiveOnly,
}

impl std::fmt::Display for DriveRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriveRole::Normal => write!(f, "normal"),
            DriveRole::ArchiveAssist => write!(f, "archive_assist"),
            DriveRole::ArchiveOnly => write!(f, "archive_only"),
        }
    }
}

impl std::str::FromStr for DriveRole {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(DriveRole::Normal),
            "archive_assist" | "archive-assist" => Ok(DriveRole::ArchiveAssist),
            "archive_only" | "archive-only" => Ok(DriveRole::ArchiveOnly),
            _ => Err(format!("unknown drive role: {s}")),
        }
    }
}

/// A drive known to Diffr.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drive {
    pub id: DriveId,
    pub identity: DriveIdentity,
    pub label: Option<String>,
    pub mount_point: PathBuf,
    /// Optional sync root directory. When set, only this directory is scanned/synced.
    pub sync_root: Option<PathBuf>,
    pub cluster_id: Option<ClusterId>,
    pub role: DriveRole,
    pub is_primary: bool,
    pub total_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub last_seen: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Drive {
    pub fn new(identity: DriveIdentity, mount_point: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: DriveId::new(),
            identity,
            label: None,
            mount_point,
            sync_root: None,
            cluster_id: None,
            role: DriveRole::Normal,
            is_primary: false,
            total_bytes: None,
            free_bytes: None,
            last_seen: now,
            created_at: now,
        }
    }

    /// Returns the effective root for scanning/syncing.
    /// Uses `sync_root` if set, otherwise falls back to `mount_point`.
    pub fn effective_root(&self) -> &Path {
        self.sync_root.as_deref().unwrap_or(&self.mount_point)
    }
}
