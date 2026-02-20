use clap::Args;
use diffr_core::config::DiffrConfig;
use diffr_core::models::drive::{Drive, DriveRole};
use diffr_db::ops;
use diffr_scan::scanner::{ScanConfig, scan_directory};
use diffr_sync::diff::{compute_diff, diff_summary, DiffEntry};
use diffr_sync::executor::{ExecConfig, execute_plan};
use diffr_sync::topology::generate_plan;

use diffr_core::models::file_entry::FileEntry;

#[derive(Args)]
pub struct SyncArgs {
    /// Cluster name to sync
    cluster: String,

    /// Dry run — show what would happen without making changes
    #[arg(long)]
    dry_run: bool,

    /// Verify file integrity after sync with SHA-256
    #[arg(long)]
    verify: bool,

    /// Skip archiving before overwrite/delete
    #[arg(long)]
    no_archive: bool,
}

pub fn run(args: SyncArgs, json: bool) -> anyhow::Result<()> {
    let db_path = DiffrConfig::db_path()?;
    let conn = diffr_db::open_db(&db_path)?;

    let cluster = ops::get_cluster_by_name(&conn, &args.cluster)?
        .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", args.cluster))?;

    let drives = ops::list_drives_for_cluster(&conn, &cluster.id)?;
    if drives.len() < 2 {
        anyhow::bail!(
            "cluster '{}' needs at least 2 drives to sync (has {})",
            cluster.name,
            drives.len()
        );
    }

    // Filter to syncable drives (not ArchiveOnly)
    let sync_drives: Vec<&Drive> = drives
        .iter()
        .filter(|d| d.role != DriveRole::ArchiveOnly)
        .collect();

    if sync_drives.len() < 2 {
        anyhow::bail!("cluster '{}' needs at least 2 syncable drives", cluster.name);
    }

    if !json {
        println!(
            "Syncing cluster '{}' ({} drives)...",
            cluster.name,
            sync_drives.len()
        );
        if args.dry_run {
            println!("  [DRY RUN]");
        }
    }

    // Scan all drives
    let mut scans: Vec<(usize, Vec<FileEntry>)> = Vec::new();
    for (idx, drive) in sync_drives.iter().enumerate() {
        let scan_root = drive.effective_root();
        if !scan_root.exists() {
            anyhow::bail!(
                "sync root does not exist: {} (drive {})",
                scan_root.display(),
                drive.identity.identity_string()
            );
        }
        if !json {
            println!("  Scanning {}...", scan_root.display());
        }
        let config = ScanConfig {
            root: scan_root.to_path_buf(),
            drive_id: drive.id.clone(),
            follow_symlinks: false,
            show_progress: !json,
        };
        let result = scan_directory(&config)?;
        scans.push((idx, result.entries));
    }

    // Compute diffs for each pair
    let mut plan_diffs: Vec<(&Drive, &Drive, Vec<DiffEntry>)> = Vec::new();
    for i in 0..scans.len() {
        for j in (i + 1)..scans.len() {
            let left_drive = sync_drives[scans[i].0];
            let right_drive = sync_drives[scans[j].0];
            let diffs = compute_diff(&scans[i].1, &scans[j].1);
            let summary = diff_summary(&diffs);

            if !json {
                println!(
                    "  {} vs {}: {}",
                    left_drive.effective_root().display(),
                    right_drive.effective_root().display(),
                    summary
                );
            }

            plan_diffs.push((left_drive, right_drive, diffs));
        }
    }

    let plan = generate_plan(&cluster, &drives, &plan_diffs);

    if !json {
        println!(
            "\nSync plan: {} operations, {} bytes total",
            plan.op_count(),
            plan.total_bytes
        );
    }

    if plan.operations.is_empty() {
        if json {
            println!("{{\"status\": \"up_to_date\"}}");
        } else {
            println!("Everything is up to date!");
        }
        return Ok(());
    }

    // Execute
    let exec_config = ExecConfig {
        dry_run: args.dry_run,
        verify: args.verify,
        archive: !args.no_archive,
        show_progress: !json,
    };

    let record = execute_plan(&plan, &drives, &exec_config)?;

    // Save sync record
    ops::insert_sync_record(&conn, &record)?;

    if json {
        println!(
            "{{\"status\": \"{}\", \"files_synced\": {}, \"bytes_transferred\": {}, \"errors\": {}}}",
            record.status, record.files_synced, record.bytes_transferred, record.errors.len()
        );
    } else {
        println!("\nSync complete:");
        println!("  Status:   {}", record.status);
        println!("  Files:    {}", record.files_synced);
        println!("  Bytes:    {}", record.bytes_transferred);
        if !record.errors.is_empty() {
            println!("  Errors:   {}", record.errors.len());
            for e in &record.errors {
                println!("    - {}", e);
            }
        }
    }

    Ok(())
}
