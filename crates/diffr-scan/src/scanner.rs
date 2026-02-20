use chrono::{DateTime, Utc};
use diffr_core::models::drive::DriveId;
use diffr_core::models::file_entry::FileEntry;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashSet;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Configuration for a scan operation.
pub struct ScanConfig {
    /// Root directory to scan.
    pub root: PathBuf,
    /// Drive ID to associate with entries.
    pub drive_id: DriveId,
    /// Whether to follow symlinks.
    pub follow_symlinks: bool,
    /// Whether to show a progress bar.
    pub show_progress: bool,
}

/// Result of scanning a directory tree.
pub struct ScanResult {
    pub entries: Vec<FileEntry>,
    pub total_files: u64,
    pub total_dirs: u64,
    pub total_bytes: u64,
    pub errors: Vec<String>,
}

/// Load ignore patterns from `.diffrignore` file.
fn load_ignore_patterns(root: &Path) -> HashSet<String> {
    let ignore_path = root.join(".diffrignore");
    let mut patterns = HashSet::new();

    // Always ignore the .diffr directory itself
    patterns.insert(".diffr".to_string());

    if let Ok(file) = fs::File::open(&ignore_path) {
        let reader = io::BufReader::new(file);
        for line in reader.lines() {
            if let Ok(line) = line {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    patterns.insert(trimmed.to_string());
                }
            }
        }
    }

    patterns
}

/// Check if a path component matches any ignore pattern.
fn should_ignore(rel_path: &Path, patterns: &HashSet<String>) -> bool {
    for component in rel_path.components() {
        let name = component.as_os_str().to_string_lossy();
        if patterns.contains(name.as_ref()) {
            return true;
        }
    }

    // Also check full relative path
    let rel_str = rel_path.to_string_lossy();
    patterns.contains(rel_str.as_ref())
}

/// Scan a directory tree and return all file entries.
pub fn scan_directory(config: &ScanConfig) -> anyhow::Result<ScanResult> {
    let ignore_patterns = load_ignore_patterns(&config.root);

    let pb = if config.show_progress {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap(),
        );
        pb.set_message("Scanning files...");
        Some(pb)
    } else {
        None
    };

    let mut entries = Vec::new();
    let mut total_files = 0u64;
    let mut total_dirs = 0u64;
    let mut total_bytes = 0u64;
    let mut errors = Vec::new();

    let walker = WalkDir::new(&config.root)
        .follow_links(config.follow_symlinks)
        .into_iter();

    for entry in walker {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                let rel_path = match path.strip_prefix(&config.root) {
                    Ok(p) => p.to_path_buf(),
                    Err(_) => continue,
                };

                // Skip the root itself
                if rel_path == Path::new("") {
                    continue;
                }

                // Check ignore patterns
                if should_ignore(&rel_path, &ignore_patterns) {
                    continue;
                }

                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(e) => {
                        errors.push(format!("{}: {}", rel_path.display(), e));
                        continue;
                    }
                };

                let is_dir = metadata.is_dir();
                let size = if is_dir { 0 } else { metadata.len() };
                let mtime = metadata
                    .modified()
                    .ok()
                    .and_then(|t| {
                        let duration = t
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        DateTime::from_timestamp(duration.as_secs() as i64, duration.subsec_nanos())
                    })
                    .unwrap_or_else(Utc::now);

                if is_dir {
                    total_dirs += 1;
                } else {
                    total_files += 1;
                    total_bytes += size;
                }

                entries.push(FileEntry {
                    rel_path,
                    drive_id: config.drive_id.clone(),
                    is_dir,
                    size,
                    mtime,
                    xxh3_hash: None,
                    sha256_hash: None,
                    indexed_at: Utc::now(),
                });

                if let Some(ref pb) = pb {
                    pb.set_message(format!(
                        "{} files, {} dirs scanned",
                        total_files, total_dirs
                    ));
                    pb.tick();
                }
            }
            Err(e) => {
                errors.push(format!("walk error: {}", e));
            }
        }
    }

    if let Some(pb) = pb {
        pb.finish_with_message(format!(
            "Scanned {} files, {} dirs ({} bytes)",
            total_files, total_dirs, total_bytes
        ));
    }

    Ok(ScanResult {
        entries,
        total_files,
        total_dirs,
        total_bytes,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use diffr_core::models::drive::DriveId;
    use tempfile::TempDir;

    #[test]
    fn test_scan_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("file1.txt"), "hello").unwrap();
        fs::write(dir.path().join("file2.txt"), "world").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir/file3.txt"), "nested").unwrap();

        let config = ScanConfig {
            root: dir.path().to_path_buf(),
            drive_id: DriveId::new(),
            follow_symlinks: false,
            show_progress: false,
        };

        let result = scan_directory(&config).unwrap();
        assert_eq!(result.total_files, 3);
        assert_eq!(result.total_dirs, 1);
    }

    #[test]
    fn test_diffrignore() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".diffrignore"), "ignore_me\n# comment\n").unwrap();
        fs::write(dir.path().join("keep.txt"), "keep").unwrap();
        fs::create_dir(dir.path().join("ignore_me")).unwrap();
        fs::write(dir.path().join("ignore_me/secret.txt"), "secret").unwrap();

        let config = ScanConfig {
            root: dir.path().to_path_buf(),
            drive_id: DriveId::new(),
            follow_symlinks: false,
            show_progress: false,
        };

        let result = scan_directory(&config).unwrap();
        // Should only find keep.txt and .diffrignore
        assert!(result
            .entries
            .iter()
            .all(|e| !e.rel_path.starts_with("ignore_me")));
    }
}
