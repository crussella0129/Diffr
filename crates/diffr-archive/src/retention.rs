use chrono::Utc;
use diffr_core::models::archive::{ArchiveEntry, RetentionPolicy};
use diffr_core::models::drive::DriveId;
use diffr_db::ops;
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::Path;

/// Result of enforcing retention policies.
#[derive(Debug, Default)]
pub struct RetentionResult {
    pub entries_pruned: usize,
    pub bytes_freed: u64,
    pub errors: Vec<String>,
}

/// Enforce retention policies on archives for a given drive.
pub fn enforce_retention(
    conn: &Connection,
    drive_id: &DriveId,
    drive_root: &Path,
    policy: &RetentionPolicy,
) -> anyhow::Result<RetentionResult> {
    let archives = ops::list_archives_for_drive(conn, drive_id)?;
    let mut result = RetentionResult::default();

    // Group archives by original path
    let mut by_path: HashMap<String, Vec<ArchiveEntry>> = HashMap::new();
    for entry in archives {
        let key = entry.original_path.to_string_lossy().to_string();
        by_path.entry(key).or_default().push(entry);
    }

    // Sort each group by archived_at descending (newest first)
    for entries in by_path.values_mut() {
        entries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
    }

    let now = Utc::now();
    let mut to_delete: Vec<ArchiveEntry> = Vec::new();

    for (_path, entries) in &by_path {
        for (i, entry) in entries.iter().enumerate() {
            let mut should_prune = false;

            // Check max_versions
            if let Some(max_versions) = policy.max_versions {
                if i >= max_versions as usize {
                    should_prune = true;
                }
            }

            // Check max_age_days
            if let Some(max_age_days) = policy.max_age_days {
                let age = now.signed_duration_since(entry.archived_at);
                if age.num_days() > max_age_days as i64 {
                    should_prune = true;
                }
            }

            if should_prune {
                to_delete.push(entry.clone());
            }
        }
    }

    // Check max_total_bytes
    if let Some(max_total) = policy.max_total_bytes {
        let current_total = ops::get_total_archive_size(conn, drive_id)?;
        if current_total > max_total {
            // Delete oldest entries until we're under the limit
            let mut all_entries: Vec<ArchiveEntry> = by_path
                .values()
                .flatten()
                .cloned()
                .collect();
            all_entries.sort_by(|a, b| a.archived_at.cmp(&b.archived_at));

            let mut freed = 0u64;
            let excess = current_total - max_total;
            for entry in all_entries {
                if freed >= excess {
                    break;
                }
                if !to_delete.iter().any(|d| d.id == entry.id) {
                    freed += entry.compressed_size;
                    to_delete.push(entry);
                }
            }
        }
    }

    // Execute deletions
    for entry in &to_delete {
        let archive_full = drive_root.join(&entry.archive_path);
        if archive_full.exists() {
            match std::fs::remove_file(&archive_full) {
                Ok(()) => {
                    result.bytes_freed += entry.compressed_size;
                }
                Err(e) => {
                    result.errors.push(format!(
                        "failed to delete {}: {}",
                        archive_full.display(),
                        e
                    ));
                    continue;
                }
            }
        }
        match ops::delete_archive(conn, &entry.id) {
            Ok(()) => {
                result.entries_pruned += 1;
            }
            Err(e) => {
                result.errors.push(format!(
                    "failed to delete archive record {}: {}",
                    entry.id, e
                ));
            }
        }
    }

    Ok(result)
}
