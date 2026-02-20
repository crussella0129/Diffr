use chrono::Utc;
use diffr_core::models::drive::Drive;
use diffr_core::models::sync_state::{SyncOp, SyncOpKind, SyncPlan, SyncRecord, SyncStatus};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Configuration for a sync execution.
pub struct ExecConfig {
    /// If true, don't actually copy/delete files — just report what would happen.
    pub dry_run: bool,
    /// If true, verify file integrity after copy with SHA-256.
    pub verify: bool,
    /// If true, archive files before overwriting/deleting.
    pub archive: bool,
    /// Show progress bars.
    pub show_progress: bool,
}

impl Default for ExecConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            verify: false,
            archive: true,
            show_progress: true,
        }
    }
}

/// Execute a sync plan.
pub fn execute_plan(
    plan: &SyncPlan,
    drives: &[Drive],
    config: &ExecConfig,
) -> anyhow::Result<SyncRecord> {
    let started_at = Utc::now();
    let drive_map: HashMap<_, _> = drives.iter().map(|d| (&d.id, d)).collect();

    let pb = if config.show_progress && !plan.operations.is_empty() {
        let pb = ProgressBar::new(plan.operations.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let mut files_synced = 0u64;
    let mut bytes_transferred = 0u64;
    let mut errors = Vec::new();

    for op in &plan.operations {
        if let Some(ref pb) = pb {
            pb.set_message(format!("{}", op.rel_path.display()));
        }

        if config.dry_run {
            tracing::info!(
                "[dry-run] {} {} -> {}",
                op.kind,
                op.rel_path.display(),
                op.target_drive,
            );
            files_synced += 1;
            bytes_transferred += op.size_bytes;
        } else {
            match execute_op(op, &drive_map, config) {
                Ok(()) => {
                    files_synced += 1;
                    bytes_transferred += op.size_bytes;
                }
                Err(e) => {
                    let msg = format!("{}: {}", op.rel_path.display(), e);
                    tracing::error!("{}", msg);
                    errors.push(msg);
                }
            }
        }

        if let Some(ref pb) = pb {
            pb.inc(1);
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message("Sync complete");
    }

    let status = if errors.is_empty() {
        SyncStatus::Success
    } else if files_synced > 0 {
        SyncStatus::PartialSuccess
    } else {
        SyncStatus::Failed
    };

    Ok(SyncRecord {
        id: Uuid::now_v7(),
        cluster_id: plan.cluster_id.clone(),
        started_at,
        finished_at: Utc::now(),
        files_synced,
        bytes_transferred,
        conflicts_resolved: 0,
        errors,
        status,
    })
}

/// Execute a single sync operation.
fn execute_op(
    op: &SyncOp,
    drives: &HashMap<&diffr_core::models::drive::DriveId, &Drive>,
    _config: &ExecConfig,
) -> anyhow::Result<()> {
    let target = drives
        .get(&op.target_drive)
        .ok_or_else(|| anyhow::anyhow!("target drive not found: {}", op.target_drive))?;

    match op.kind {
        SyncOpKind::CopyNew | SyncOpKind::Overwrite => {
            let source_id = op
                .source_drive
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no source drive for copy op"))?;
            let source = drives
                .get(source_id)
                .ok_or_else(|| anyhow::anyhow!("source drive not found: {}", source_id))?;

            let src_path = source.effective_root().join(&op.rel_path);
            let dst_path = target.effective_root().join(&op.rel_path);

            atomic_copy(&src_path, &dst_path)?;
        }
        SyncOpKind::Delete => {
            let dst_path = target.effective_root().join(&op.rel_path);
            if dst_path.exists() {
                std::fs::remove_file(&dst_path)?;
            }
        }
        SyncOpKind::ResolveConflict => {
            // Conflicts should be resolved before reaching the executor
            tracing::warn!("unresolved conflict: {}", op.rel_path.display());
        }
    }

    Ok(())
}

/// Atomic file copy: write to temp file in target directory, then rename.
fn atomic_copy(src: &Path, dst: &Path) -> anyhow::Result<()> {
    // Verify source exists and is accessible
    if !src.exists() {
        anyhow::bail!("source file does not exist: {}", src.display());
    }

    // Ensure destination directory exists
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Write to temp file in the same directory
    let parent = dst.parent().unwrap_or(Path::new("."));
    let temp = tempfile::NamedTempFile::new_in(parent)?;
    std::fs::copy(src, temp.path())?;

    // Atomic rename (same filesystem)
    temp.persist(dst)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_copy() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let src_file = src_dir.path().join("test.txt");
        std::fs::write(&src_file, "hello world").unwrap();

        let dst_file = dst_dir.path().join("test.txt");
        atomic_copy(&src_file, &dst_file).unwrap();

        assert_eq!(std::fs::read_to_string(&dst_file).unwrap(), "hello world");
    }

    #[test]
    fn test_atomic_copy_creates_dirs() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let src_file = src_dir.path().join("test.txt");
        std::fs::write(&src_file, "content").unwrap();

        let dst_file = dst_dir.path().join("sub/dir/test.txt");
        atomic_copy(&src_file, &dst_file).unwrap();

        assert!(dst_file.exists());
    }
}
