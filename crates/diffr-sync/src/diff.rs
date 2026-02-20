use diffr_core::models::file_entry::FileEntry;
use std::collections::HashMap;
use std::path::PathBuf;

/// The result of comparing two file trees.
#[derive(Debug, Clone)]
pub struct DiffEntry {
    pub rel_path: PathBuf,
    pub kind: DiffKind,
    pub left: Option<FileEntry>,
    pub right: Option<FileEntry>,
}

/// Classification of a diff entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffKind {
    /// File exists only on the left drive.
    OnlyLeft,
    /// File exists only on the right drive.
    OnlyRight,
    /// File exists on both but differs (size, mtime, or hash).
    Modified,
    /// Both sides modified since last sync — conflict.
    Conflict,
    /// Files are identical.
    Identical,
}

impl std::fmt::Display for DiffKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiffKind::OnlyLeft => write!(f, "only_left"),
            DiffKind::OnlyRight => write!(f, "only_right"),
            DiffKind::Modified => write!(f, "modified"),
            DiffKind::Conflict => write!(f, "conflict"),
            DiffKind::Identical => write!(f, "identical"),
        }
    }
}

/// Compare two sets of file entries and produce a diff.
///
/// `left` and `right` are the file entries from two different drives.
/// Entries are matched by relative path.
pub fn compute_diff(left: &[FileEntry], right: &[FileEntry]) -> Vec<DiffEntry> {
    let left_map: HashMap<&PathBuf, &FileEntry> =
        left.iter().map(|e| (&e.rel_path, e)).collect();
    let right_map: HashMap<&PathBuf, &FileEntry> =
        right.iter().map(|e| (&e.rel_path, e)).collect();

    let mut diffs = Vec::new();

    // Check all left entries
    for (path, left_entry) in &left_map {
        match right_map.get(path) {
            Some(right_entry) => {
                let kind = classify_pair(left_entry, right_entry);
                diffs.push(DiffEntry {
                    rel_path: (*path).clone(),
                    kind,
                    left: Some((*left_entry).clone()),
                    right: Some((*right_entry).clone()),
                });
            }
            None => {
                diffs.push(DiffEntry {
                    rel_path: (*path).clone(),
                    kind: DiffKind::OnlyLeft,
                    left: Some((*left_entry).clone()),
                    right: None,
                });
            }
        }
    }

    // Check right-only entries
    for (path, right_entry) in &right_map {
        if !left_map.contains_key(path) {
            diffs.push(DiffEntry {
                rel_path: (*path).clone(),
                kind: DiffKind::OnlyRight,
                left: None,
                right: Some((*right_entry).clone()),
            });
        }
    }

    diffs.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    diffs
}

/// Classify a pair of files that exist on both drives.
fn classify_pair(left: &FileEntry, right: &FileEntry) -> DiffKind {
    // Skip directories
    if left.is_dir && right.is_dir {
        return DiffKind::Identical;
    }

    // If hashes are available, compare by hash
    if let (Some(lh), Some(rh)) = (&left.xxh3_hash, &right.xxh3_hash) {
        if lh == rh {
            return DiffKind::Identical;
        }
        // Different hashes — check if it's a conflict or one-way modification
        // For now, treat as Modified (conflict detection needs sync history)
        return DiffKind::Modified;
    }

    // Fall back to metadata comparison
    if left.size == right.size && left.mtime == right.mtime {
        DiffKind::Identical
    } else {
        DiffKind::Modified
    }
}

/// Count the diff entries by kind.
pub fn diff_summary(diffs: &[DiffEntry]) -> DiffSummary {
    let mut summary = DiffSummary::default();
    for d in diffs {
        match d.kind {
            DiffKind::OnlyLeft => summary.only_left += 1,
            DiffKind::OnlyRight => summary.only_right += 1,
            DiffKind::Modified => summary.modified += 1,
            DiffKind::Conflict => summary.conflicts += 1,
            DiffKind::Identical => summary.identical += 1,
        }
    }
    summary
}

#[derive(Debug, Default)]
pub struct DiffSummary {
    pub only_left: usize,
    pub only_right: usize,
    pub modified: usize,
    pub conflicts: usize,
    pub identical: usize,
}

impl DiffSummary {
    pub fn has_changes(&self) -> bool {
        self.only_left > 0 || self.only_right > 0 || self.modified > 0 || self.conflicts > 0
    }

    pub fn total_changes(&self) -> usize {
        self.only_left + self.only_right + self.modified + self.conflicts
    }
}

impl std::fmt::Display for DiffSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} identical, {} left-only, {} right-only, {} modified, {} conflicts",
            self.identical, self.only_left, self.only_right, self.modified, self.conflicts
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use diffr_core::models::drive::DriveId;

    fn make_entry(path: &str, drive_id: &DriveId, size: u64) -> FileEntry {
        FileEntry {
            rel_path: PathBuf::from(path),
            drive_id: drive_id.clone(),
            is_dir: false,
            size,
            mtime: Utc::now(),
            xxh3_hash: None,
            sha256_hash: None,
            indexed_at: Utc::now(),
        }
    }

    #[test]
    fn test_diff_only_left() {
        let d1 = DriveId::new();
        let left = vec![make_entry("a.txt", &d1, 100)];
        let right: Vec<FileEntry> = vec![];
        let diffs = compute_diff(&left, &right);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::OnlyLeft);
    }

    #[test]
    fn test_diff_identical_by_metadata() {
        let d1 = DriveId::new();
        let d2 = DriveId::new();
        let mtime = Utc::now();
        let left = vec![FileEntry {
            rel_path: "a.txt".into(),
            drive_id: d1.clone(),
            is_dir: false,
            size: 100,
            mtime,
            xxh3_hash: None,
            sha256_hash: None,
            indexed_at: Utc::now(),
        }];
        let right = vec![FileEntry {
            rel_path: "a.txt".into(),
            drive_id: d2.clone(),
            is_dir: false,
            size: 100,
            mtime,
            xxh3_hash: None,
            sha256_hash: None,
            indexed_at: Utc::now(),
        }];
        let diffs = compute_diff(&left, &right);
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].kind, DiffKind::Identical);
    }
}
