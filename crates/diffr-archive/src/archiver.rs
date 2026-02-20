use chrono::Utc;
use diffr_core::models::archive::{ArchiveEntry, ArchiveReason, CompressionFormat};
use diffr_core::models::drive::{Drive, DriveRole};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Archive a file before it is overwritten or deleted.
pub fn archive_file(
    drive: &Drive,
    rel_path: &Path,
    reason: ArchiveReason,
) -> anyhow::Result<ArchiveEntry> {
    let source_path = drive.effective_root().join(rel_path);
    if !source_path.exists() {
        anyhow::bail!("source file does not exist: {}", source_path.display());
    }

    let metadata = std::fs::metadata(&source_path)?;
    let original_size = metadata.len();

    // Determine compression based on drive role
    let compression = match drive.role {
        DriveRole::ArchiveOnly | DriveRole::ArchiveAssist => CompressionFormat::Zstd,
        DriveRole::Normal => CompressionFormat::Zstd,
    };

    // Build archive path: .diffr/archive/<rel_path>/<timestamp>.zst
    let archive_id = Uuid::now_v7();
    let timestamp = Utc::now().format("%Y%m%dT%H%M%S");
    let ext = match compression {
        CompressionFormat::Zstd => ".zst",
        CompressionFormat::None => "",
    };
    let archive_rel = PathBuf::from(".diffr")
        .join("archive")
        .join(rel_path)
        .join(format!("{}{}", timestamp, ext));
    let archive_path = drive.effective_root().join(&archive_rel);

    // Create archive directory
    if let Some(parent) = archive_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Compress and write
    let compressed_size = match compression {
        CompressionFormat::Zstd => compress_zstd(&source_path, &archive_path)?,
        CompressionFormat::None => {
            std::fs::copy(&source_path, &archive_path)?;
            original_size
        }
    };

    // Compute hash of original file for verification
    let data = std::fs::read(&source_path)?;
    let xxh3_hash = format!("{:016x}", xxhash_rust::xxh3::xxh3_64(&data));

    Ok(ArchiveEntry {
        id: archive_id,
        original_path: rel_path.to_path_buf(),
        archive_path: archive_rel,
        drive_id: drive.id.clone(),
        original_size,
        compressed_size,
        compression,
        xxh3_hash,
        reason,
        archived_at: Utc::now(),
    })
}

/// Compress a file using zstd.
fn compress_zstd(src: &Path, dst: &Path) -> anyhow::Result<u64> {
    let input = std::fs::read(src)?;
    let compressed = zstd::encode_all(input.as_slice(), 3)?;
    let size = compressed.len() as u64;
    std::fs::write(dst, &compressed)?;
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use diffr_core::models::drive::{Drive, DriveIdentity};
    use tempfile::TempDir;

    #[test]
    fn test_archive_file() {
        let dir = TempDir::new().unwrap();
        let test_file = dir.path().join("test.txt");
        std::fs::write(&test_file, "hello world, this is a test file for archiving").unwrap();

        let drive = Drive::new(
            DriveIdentity::new_synthetic(),
            dir.path().to_path_buf(),
        );

        let entry = archive_file(&drive, Path::new("test.txt"), ArchiveReason::BeforeOverwrite)
            .unwrap();

        assert_eq!(entry.original_path, PathBuf::from("test.txt"));
        assert!(entry.compressed_size > 0);
        assert_eq!(entry.compression, CompressionFormat::Zstd);

        // Verify archive file exists on disk
        let archive_full = dir.path().join(&entry.archive_path);
        assert!(archive_full.exists());
    }
}
