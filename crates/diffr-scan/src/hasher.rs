use indicatif::{ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;
use xxhash_rust::xxh3::xxh3_64;

/// Hash result containing both fast and verification hashes.
#[derive(Debug, Clone)]
pub struct HashResult {
    pub xxh3_hex: String,
    pub sha256_hex: Option<String>,
}

/// Compute the XXH3-64 hash of a file.
pub fn xxh3_file(path: &Path) -> anyhow::Result<String> {
    let data = std::fs::read(path)?;
    let hash = xxh3_64(&data);
    Ok(format!("{:016x}", hash))
}

/// Compute the SHA-256 hash of a file.
pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

/// Compute both XXH3 and optionally SHA-256 hash of a file.
pub fn hash_file(path: &Path, include_sha256: bool) -> anyhow::Result<HashResult> {
    let data = std::fs::read(path)?;
    let xxh3_hex = format!("{:016x}", xxh3_64(&data));

    let sha256_hex = if include_sha256 {
        let mut hasher = Sha256::new();
        hasher.update(&data);
        Some(format!("{:x}", hasher.finalize()))
    } else {
        None
    };

    Ok(HashResult {
        xxh3_hex,
        sha256_hex,
    })
}

/// Bulk hash a list of files with optional progress display.
pub fn hash_files_bulk(
    root: &Path,
    rel_paths: &[&Path],
    include_sha256: bool,
    show_progress: bool,
) -> Vec<(usize, anyhow::Result<HashResult>)> {
    let pb = if show_progress {
        let pb = ProgressBar::new(rel_paths.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} files hashed ({eta})")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    } else {
        None
    };

    let results: Vec<_> = rel_paths
        .iter()
        .enumerate()
        .map(|(i, rel_path)| {
            let full_path = root.join(rel_path);
            let result = hash_file(&full_path, include_sha256);
            if let Some(ref pb) = pb {
                pb.inc(1);
            }
            (i, result)
        })
        .collect();

    if let Some(pb) = pb {
        pb.finish_with_message("Hashing complete");
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_xxh3_deterministic() {
        let mut f = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"hello world").unwrap();
        let h1 = xxh3_file(f.path()).unwrap();
        let h2 = xxh3_file(f.path()).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16); // 64-bit = 16 hex chars
    }

    #[test]
    fn test_sha256_known() {
        let mut f = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"hello world").unwrap();
        let h = sha256_file(f.path()).unwrap();
        assert_eq!(
            h,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_hash_file_both() {
        let mut f = NamedTempFile::new().unwrap();
        std::io::Write::write_all(&mut f, b"test data").unwrap();
        let result = hash_file(f.path(), true).unwrap();
        assert!(!result.xxh3_hex.is_empty());
        assert!(result.sha256_hex.is_some());
    }
}
