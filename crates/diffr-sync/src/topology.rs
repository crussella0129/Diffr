use diffr_core::models::cluster::{Cluster, Topology};
use diffr_core::models::drive::Drive;
use diffr_core::models::sync_state::{SyncOp, SyncOpKind, SyncPlan};
use uuid::Uuid;

use crate::diff::{DiffEntry, DiffKind};

/// Generate a sync plan based on cluster topology and diff results.
pub fn generate_plan(
    cluster: &Cluster,
    drives: &[Drive],
    diffs_per_pair: &[(&Drive, &Drive, Vec<DiffEntry>)],
) -> SyncPlan {
    let mut operations = Vec::new();

    match cluster.topology {
        Topology::Mesh => {
            generate_mesh_ops(&mut operations, diffs_per_pair);
        }
        Topology::PrimaryReplica => {
            generate_primary_replica_ops(&mut operations, drives, diffs_per_pair);
        }
    }

    SyncPlan::new(cluster.id.clone(), operations)
}

/// Mesh topology: changes flow in all directions. Each missing/modified file
/// is copied to the drive that doesn't have the latest version.
fn generate_mesh_ops(
    operations: &mut Vec<SyncOp>,
    diffs: &[(&Drive, &Drive, Vec<DiffEntry>)],
) {
    for (left_drive, right_drive, diff_entries) in diffs {
        for entry in diff_entries {
            match entry.kind {
                DiffKind::OnlyLeft => {
                    // Copy from left to right
                    let size = entry.left.as_ref().map(|e| e.size).unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::CopyNew,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(left_drive.id.clone()),
                        target_drive: right_drive.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::OnlyRight => {
                    // Copy from right to left
                    let size = entry.right.as_ref().map(|e| e.size).unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::CopyNew,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(right_drive.id.clone()),
                        target_drive: left_drive.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::Modified => {
                    // Newer file wins; copy to the other drive
                    let (source, target, size) = pick_newer(left_drive, right_drive, entry);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::Overwrite,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(source.id.clone()),
                        target_drive: target.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::Conflict => {
                    let size = entry
                        .left
                        .as_ref()
                        .or(entry.right.as_ref())
                        .map(|e| e.size)
                        .unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::ResolveConflict,
                        rel_path: entry.rel_path.clone(),
                        source_drive: None,
                        target_drive: right_drive.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::Identical => {} // Nothing to do
            }
        }
    }
}

/// Primary/replica: only the primary's files are authoritative.
fn generate_primary_replica_ops(
    operations: &mut Vec<SyncOp>,
    drives: &[Drive],
    diffs: &[(&Drive, &Drive, Vec<DiffEntry>)],
) {
    let primary = drives.iter().find(|d| d.is_primary);

    for (left_drive, right_drive, diff_entries) in diffs {
        for entry in diff_entries {
            // Determine which side is primary
            let left_is_primary = primary.map(|p| p.id == left_drive.id).unwrap_or(false);

            match entry.kind {
                DiffKind::OnlyLeft if left_is_primary => {
                    let size = entry.left.as_ref().map(|e| e.size).unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::CopyNew,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(left_drive.id.clone()),
                        target_drive: right_drive.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::OnlyRight if !left_is_primary => {
                    let size = entry.right.as_ref().map(|e| e.size).unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::CopyNew,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(right_drive.id.clone()),
                        target_drive: left_drive.id.clone(),
                        size_bytes: size,
                    });
                }
                DiffKind::Modified | DiffKind::Conflict => {
                    // Primary always wins in primary/replica topology
                    let (source, target) = if left_is_primary {
                        (left_drive, right_drive)
                    } else {
                        (right_drive, left_drive)
                    };
                    let size = entry
                        .left
                        .as_ref()
                        .or(entry.right.as_ref())
                        .map(|e| e.size)
                        .unwrap_or(0);
                    operations.push(SyncOp {
                        id: Uuid::now_v7(),
                        kind: SyncOpKind::Overwrite,
                        rel_path: entry.rel_path.clone(),
                        source_drive: Some(source.id.clone()),
                        target_drive: target.id.clone(),
                        size_bytes: size,
                    });
                }
                _ => {} // OnlyLeft on replica side, OnlyRight on primary side — skip
            }
        }
    }
}

/// Pick the newer file based on mtime.
fn pick_newer<'a>(
    left_drive: &'a Drive,
    right_drive: &'a Drive,
    entry: &DiffEntry,
) -> (&'a Drive, &'a Drive, u64) {
    let left_mtime = entry.left.as_ref().map(|e| e.mtime);
    let right_mtime = entry.right.as_ref().map(|e| e.mtime);

    match (left_mtime, right_mtime) {
        (Some(l), Some(r)) if l >= r => {
            let size = entry.left.as_ref().map(|e| e.size).unwrap_or(0);
            (left_drive, right_drive, size)
        }
        _ => {
            let size = entry.right.as_ref().map(|e| e.size).unwrap_or(0);
            (right_drive, left_drive, size)
        }
    }
}
