use clap::Subcommand;
use diffr_core::config::DiffrConfig;
use diffr_core::models::drive::DriveIdentity;
use diffr_db::ops;

#[derive(Subcommand)]
pub enum ArchiveAction {
    /// List archived versions
    List {
        /// Filter by original file path
        #[arg(long)]
        path: Option<String>,
        /// Filter by drive identity
        #[arg(long)]
        drive: Option<String>,
    },
    /// Restore a file from the archive
    Restore {
        /// Archive entry ID
        id: String,
        /// Destination path (defaults to original location)
        #[arg(long)]
        dest: Option<String>,
    },
    /// Prune old archives according to retention policy
    Prune {
        /// Drive identity to prune archives from
        drive: String,
    },
}

pub fn run(action: ArchiveAction, json: bool) -> anyhow::Result<()> {
    let db_path = DiffrConfig::db_path()?;
    let conn = diffr_db::open_db(&db_path)?;

    match action {
        ArchiveAction::List { path, drive } => {
            let archives = if let Some(path) = &path {
                ops::list_archives_for_path(&conn, path)?
            } else if let Some(drive_serial) = &drive {
                let identity = DriveIdentity::Hardware {
                    serial: drive_serial.clone(),
                };
                let drive = ops::get_drive_by_identity(&conn, &identity)?
                    .ok_or_else(|| anyhow::anyhow!("drive '{}' not found", drive_serial))?;
                ops::list_archives_for_drive(&conn, &drive.id)?
            } else {
                anyhow::bail!("specify --path or --drive to filter archives");
            };

            if json {
                let items: Vec<_> = archives
                    .iter()
                    .map(|a| {
                        format!(
                            "{{\"id\": \"{}\", \"path\": \"{}\", \"size\": {}, \"compressed\": {}, \"archived_at\": \"{}\"}}",
                            a.id, a.original_path.display(), a.original_size, a.compressed_size, a.archived_at
                        )
                    })
                    .collect();
                println!("[{}]", items.join(", "));
            } else {
                if archives.is_empty() {
                    println!("No archived versions found.");
                } else {
                    println!(
                        "{:<36} {:<30} {:>10} {:>10} {:<20}",
                        "ID", "PATH", "ORIGINAL", "COMPRESSED", "ARCHIVED"
                    );
                    for a in &archives {
                        println!(
                            "{:<36} {:<30} {:>10} {:>10} {:<20}",
                            a.id,
                            a.original_path.display(),
                            a.original_size,
                            a.compressed_size,
                            a.archived_at.format("%Y-%m-%d %H:%M:%S")
                        );
                    }
                }
            }
            Ok(())
        }
        ArchiveAction::Restore { id, dest } => {
            let archive_id: uuid::Uuid = id.parse()?;

            // Find the archive entry (search all drives)
            let drives = ops::list_all_drives(&conn)?;
            let mut found = None;
            for drive in &drives {
                let archives = ops::list_archives_for_drive(&conn, &drive.id)?;
                if let Some(entry) = archives.into_iter().find(|a| a.id == archive_id) {
                    found = Some((drive.clone(), entry));
                    break;
                }
            }

            let (drive, entry) = found
                .ok_or_else(|| anyhow::anyhow!("archive entry '{}' not found", id))?;

            let dest_path = dest.map(std::path::PathBuf::from);
            diffr_archive::retriever::restore_file(
                &drive,
                &entry,
                dest_path.as_deref(),
            )?;

            println!(
                "Restored {} from archive to {}",
                entry.original_path.display(),
                dest_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| entry.original_path.display().to_string())
            );
            Ok(())
        }
        ArchiveAction::Prune { drive } => {
            let identity = DriveIdentity::Hardware {
                serial: drive.clone(),
            };
            let drive_obj = ops::get_drive_by_identity(&conn, &identity)?
                .ok_or_else(|| anyhow::anyhow!("drive '{}' not found", drive))?;

            let config = DiffrConfig::load()?;
            let result = diffr_archive::retention::enforce_retention(
                &conn,
                &drive_obj.id,
                drive_obj.effective_root(),
                &config.retention,
            )?;

            if json {
                println!(
                    "{{\"pruned\": {}, \"bytes_freed\": {}, \"errors\": {}}}",
                    result.entries_pruned,
                    result.bytes_freed,
                    result.errors.len()
                );
            } else {
                println!(
                    "Pruned {} archive entries, freed {} bytes",
                    result.entries_pruned, result.bytes_freed
                );
                if !result.errors.is_empty() {
                    for e in &result.errors {
                        println!("  Error: {}", e);
                    }
                }
            }
            Ok(())
        }
    }
}
