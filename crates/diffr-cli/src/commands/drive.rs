use clap::Subcommand;
use diffr_core::config::DiffrConfig;
use diffr_core::models::drive::{Drive, DriveIdentity, DriveRole};
use diffr_db::ops;

#[derive(Subcommand)]
pub enum DriveAction {
    /// Scan for connected drives
    Scan,
    /// Add a drive to a cluster
    Add {
        /// Drive serial number or synthetic ID
        identity: String,
        /// Cluster to add the drive to
        #[arg(long)]
        cluster: String,
        /// Drive role: normal, archive-assist, or archive-only
        #[arg(long, default_value = "normal")]
        role: String,
        /// Mark this drive as the primary (for primary-replica topology)
        #[arg(long)]
        primary: bool,
        /// Path to a diffr repo (must have been initialized with `diffr init`)
        #[arg(long)]
        path: Option<std::path::PathBuf>,
    },
    /// Remove a drive from its cluster
    Remove {
        /// Drive serial number or synthetic ID
        identity: String,
    },
    /// List all known drives
    List,
    /// Show detailed drive info
    Info {
        /// Drive serial number or synthetic ID
        identity: String,
    },
}

pub fn run(action: DriveAction, json: bool) -> anyhow::Result<()> {
    match action {
        DriveAction::Scan => {
            let discovery = diffr_discovery::platform::get_discovery();
            let drives = discovery.discover_drives()?;

            if json {
                let items: Vec<_> = drives
                    .iter()
                    .map(|d| {
                        format!(
                            "{{\"identity\": \"{}\", \"mount\": \"{}\", \"label\": {}}}",
                            d.identity.identity_string(),
                            d.mount_point.display(),
                            d.label
                                .as_ref()
                                .map(|l| format!("\"{}\"", l))
                                .unwrap_or_else(|| "null".to_string())
                        )
                    })
                    .collect();
                println!("[{}]", items.join(", "));
            } else {
                if drives.is_empty() {
                    println!("No drives detected.");
                } else {
                    println!(
                        "{:<30} {:<20} {:<15} {:>12} {:>12}",
                        "IDENTITY", "MOUNT", "LABEL", "TOTAL", "FREE"
                    );
                    for d in &drives {
                        println!(
                            "{:<30} {:<20} {:<15} {:>12} {:>12}",
                            d.identity.identity_string(),
                            d.mount_point.display(),
                            d.label.as_deref().unwrap_or("-"),
                            d.total_bytes
                                .map(|b| format_bytes(b))
                                .unwrap_or_else(|| "-".to_string()),
                            d.free_bytes
                                .map(|b| format_bytes(b))
                                .unwrap_or_else(|| "-".to_string()),
                        );
                    }
                }
            }
            Ok(())
        }
        DriveAction::Add {
            identity,
            cluster,
            role,
            primary,
            path,
        } => {
            let db_path = DiffrConfig::db_path()?;
            let conn = diffr_db::open_db(&db_path)?;

            let cluster_obj = ops::get_cluster_by_name(&conn, &cluster)?
                .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", cluster))?;

            let role: DriveRole = role.parse().map_err(|e: String| anyhow::anyhow!(e))?;

            // Validate and canonicalize sync root path if provided
            let sync_root = if let Some(ref p) = path {
                let canon = std::fs::canonicalize(p)
                    .map_err(|_| anyhow::anyhow!("path does not exist: {}", p.display()))?;
                let repo_toml = canon.join(".diffr").join("repo.toml");
                if !repo_toml.exists() {
                    anyhow::bail!(
                        "diffr repo not initialized at {} (run `diffr init {}`)",
                        canon.display(),
                        canon.display()
                    );
                }
                Some(canon)
            } else {
                None
            };

            // Try to find the drive by discovery first
            let discovery = diffr_discovery::platform::get_discovery();
            let discovered = discovery.find_by_serial(&identity)?;

            let mut drive = match discovered {
                Some(d) => d,
                None => {
                    // Create a minimal drive entry
                    Drive::new(
                        DriveIdentity::Hardware {
                            serial: identity.clone(),
                        },
                        std::path::PathBuf::from("."),
                    )
                }
            };

            drive.cluster_id = Some(cluster_obj.id.clone());
            drive.role = role;
            drive.is_primary = primary;
            drive.sync_root = sync_root;

            // Check if already registered
            if ops::get_drive_by_identity(&conn, &drive.identity)?.is_some() {
                // Update cluster assignment
                let existing = ops::get_drive_by_identity(&conn, &drive.identity)?.unwrap();
                ops::update_drive_cluster(&conn, &existing.id, Some(&cluster_obj.id))?;
                println!(
                    "Updated drive '{}' -> cluster '{}'",
                    identity, cluster
                );
            } else {
                ops::insert_drive(&conn, &drive)?;
                println!(
                    "Added drive '{}' to cluster '{}'",
                    identity, cluster
                );
            }
            Ok(())
        }
        DriveAction::Remove { identity } => {
            let db_path = DiffrConfig::db_path()?;
            let conn = diffr_db::open_db(&db_path)?;

            let drive_identity = DriveIdentity::Hardware {
                serial: identity.clone(),
            };
            let drive = ops::get_drive_by_identity(&conn, &drive_identity)?
                .ok_or_else(|| anyhow::anyhow!("drive '{}' not found", identity))?;

            ops::delete_drive(&conn, &drive.id)?;
            println!("Removed drive '{}'", identity);
            Ok(())
        }
        DriveAction::List => {
            let db_path = DiffrConfig::db_path()?;
            let conn = diffr_db::open_db(&db_path)?;
            let drives = ops::list_all_drives(&conn)?;

            if json {
                let items: Vec<_> = drives
                    .iter()
                    .map(|d| {
                        format!(
                            "{{\"identity\": \"{}\", \"mount\": \"{}\", \"cluster\": {}, \"role\": \"{}\"}}",
                            d.identity.identity_string(),
                            d.mount_point.display(),
                            d.cluster_id
                                .as_ref()
                                .map(|c| format!("\"{}\"", c))
                                .unwrap_or_else(|| "null".to_string()),
                            d.role
                        )
                    })
                    .collect();
                println!("[{}]", items.join(", "));
            } else {
                if drives.is_empty() {
                    println!("No drives registered.");
                } else {
                    println!(
                        "{:<30} {:<20} {:<20} {:<15} {:<10}",
                        "IDENTITY", "MOUNT", "SYNC ROOT", "ROLE", "PRIMARY"
                    );
                    for d in &drives {
                        let sync_root_display = d.sync_root
                            .as_ref()
                            .map(|p| p.display().to_string())
                            .unwrap_or_else(|| "-".to_string());
                        println!(
                            "{:<30} {:<20} {:<20} {:<15} {:<10}",
                            d.identity.identity_string(),
                            d.mount_point.display(),
                            sync_root_display,
                            d.role,
                            if d.is_primary { "yes" } else { "no" }
                        );
                    }
                }
            }
            Ok(())
        }
        DriveAction::Info { identity } => {
            let db_path = DiffrConfig::db_path()?;
            let conn = diffr_db::open_db(&db_path)?;

            let drive_identity = DriveIdentity::Hardware {
                serial: identity.clone(),
            };
            let drive = ops::get_drive_by_identity(&conn, &drive_identity)?
                .ok_or_else(|| anyhow::anyhow!("drive '{}' not found", identity))?;

            if json {
                println!(
                    "{{\"id\": \"{}\", \"identity\": \"{}\", \"mount\": \"{}\", \"role\": \"{}\", \"primary\": {}}}",
                    drive.id, drive.identity.identity_string(), drive.mount_point.display(), drive.role, drive.is_primary
                );
            } else {
                println!("Drive: {}", drive.identity.identity_string());
                println!("  ID:        {}", drive.id);
                println!("  Mount:     {}", drive.mount_point.display());
                if let Some(ref sr) = drive.sync_root {
                    println!("  Sync root: {}", sr.display());
                }
                println!("  Label:     {}", drive.label.as_deref().unwrap_or("-"));
                println!("  Role:      {}", drive.role);
                println!("  Primary:   {}", drive.is_primary);
                println!(
                    "  Cluster:   {}",
                    drive
                        .cluster_id
                        .as_ref()
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| "none".to_string())
                );
                println!("  Last seen: {}", drive.last_seen);
            }
            Ok(())
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.1} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
