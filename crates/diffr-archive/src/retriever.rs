use diffr_core::models::archive::{ArchiveEntry, CompressionFormat};
use diffr_core::models::drive::Drive;
use std::path::Path;

/// Restore a file from the archive to its original location.
pub fn restore_file(
    drive: &Drive,
    entry: &ArchiveEntry,
    dest_path: Option<&Path>,
) -> anyhow::Result<()> {
    let archive_full = drive.effective_root().join(&entry.archive_path);
    if !archive_full.exists() {
        anyhow::bail!(
            "archive file does not exist: {}",
            archive_full.display()
        );
    }

    let target = match dest_path {
        Some(p) => p.to_path_buf(),
        None => drive.effective_root().join(&entry.original_path),
    };

    // Ensure target directory exists
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }

    match entry.compression {
        CompressionFormat::Zstd => decompress_zstd(&archive_full, &target)?,
        CompressionFormat::None => {
            std::fs::copy(&archive_full, &target)?;
        }
    }

    // Verify hash if possible
    let restored_data = std::fs::read(&target)?;
    let hash = format!("{:016x}", xxhash_rust::xxh3::xxh3_64(&restored_data));
    if hash != entry.xxh3_hash {
        anyhow::bail!(
            "hash mismatch after restore: expected {}, got {}",
            entry.xxh3_hash,
            hash
        );
    }

    Ok(())
}

/// Decompress a zstd file.
fn decompress_zstd(src: &Path, dst: &Path) -> anyhow::Result<()> {
    let compressed = std::fs::read(src)?;
    let decompressed = zstd::decode_all(compressed.as_slice())?;
    std::fs::write(dst, &decompressed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::archiver;
    use diffr_core::models::archive::ArchiveReason;
    use diffr_core::models::drive::{Drive, DriveIdentity};
    use tempfile::TempDir;

    #[test]
    fn test_archive_and_restore() {
        let dir = TempDir::new().unwrap();
        let original_content = "hello world, this is test content for archive/restore cycle";
        std::fs::write(dir.path().join("test.txt"), original_content).unwrap();

        let drive = Drive::new(DriveIdentity::new_synthetic(), dir.path().to_path_buf());

        // Archive
        let entry = archiver::archive_file(
            &drive,
            Path::new("test.txt"),
            ArchiveReason::BeforeOverwrite,
        )
        .unwrap();

        // Overwrite original
        std::fs::write(dir.path().join("test.txt"), "modified content").unwrap();

        // Restore
        restore_file(&drive, &entry, None).unwrap();

        // Verify
        let restored = std::fs::read_to_string(dir.path().join("test.txt")).unwrap();
        assert_eq!(restored, original_content);
    }
}
