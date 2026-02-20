use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

use super::cluster::ClusterId;
use super::drive::DriveId;

/// A single sync operation to be performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncOp {
    pub id: Uuid,
    pub kind: SyncOpKind,
    pub rel_path: PathBuf,
    pub source_drive: Option<DriveId>,
    pub target_drive: DriveId,
    pub size_bytes: u64,
}

/// The kind of sync operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncOpKind {
    /// Copy a new file to the target.
    CopyNew,
    /// Overwrite an existing file on the target.
    Overwrite,
    /// Delete a file from the target (propagate deletion).
    Delete,
    /// Resolve a conflict according to the cluster's strategy.
    ResolveConflict,
}

impl std::fmt::Display for SyncOpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncOpKind::CopyNew => write!(f, "copy_new"),
            SyncOpKind::Overwrite => write!(f, "overwrite"),
            SyncOpKind::Delete => write!(f, "delete"),
            SyncOpKind::ResolveConflict => write!(f, "resolve_conflict"),
        }
    }
}

/// A plan containing all operations for a sync session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncPlan {
    pub id: Uuid,
    pub cluster_id: ClusterId,
    pub operations: Vec<SyncOp>,
    pub total_bytes: u64,
    pub created_at: DateTime<Utc>,
}

impl SyncPlan {
    pub fn new(cluster_id: ClusterId, operations: Vec<SyncOp>) -> Self {
        let total_bytes = operations.iter().map(|op| op.size_bytes).sum();
        Self {
            id: Uuid::now_v7(),
            cluster_id,
            operations,
            total_bytes,
            created_at: Utc::now(),
        }
    }

    pub fn op_count(&self) -> usize {
        self.operations.len()
    }
}

/// Record of a completed sync session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord {
    pub id: Uuid,
    pub cluster_id: ClusterId,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub files_synced: u64,
    pub bytes_transferred: u64,
    pub conflicts_resolved: u64,
    pub errors: Vec<String>,
    pub status: SyncStatus,
}

/// Status of a completed sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncStatus {
    Success,
    PartialSuccess,
    Failed,
}

impl std::fmt::Display for SyncStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SyncStatus::Success => write!(f, "success"),
            SyncStatus::PartialSuccess => write!(f, "partial_success"),
            SyncStatus::Failed => write!(f, "failed"),
        }
    }
}

/// How a conflict was resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub rel_path: PathBuf,
    pub winner_drive: DriveId,
    pub loser_drive: DriveId,
    pub strategy_used: String,
    pub resolved_at: DateTime<Utc>,
}
