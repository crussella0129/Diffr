use clap::Args;
use diffr_core::config::DiffrConfig;
use diffr_db::ops;

#[derive(Args)]
pub struct StatusArgs {
    /// Cluster name (shows all clusters if omitted)
    cluster: Option<String>,
}

pub fn run(args: StatusArgs, json: bool) -> anyhow::Result<()> {
    let db_path = DiffrConfig::db_path()?;
    let conn = diffr_db::open_db(&db_path)?;

    let clusters = match args.cluster {
        Some(name) => {
            let c = ops::get_cluster_by_name(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", name))?;
            vec![c]
        }
        None => ops::list_clusters(&conn)?,
    };

    for cluster in &clusters {
        let drives = ops::list_drives_for_cluster(&conn, &cluster.id)?;
        let history = ops::list_sync_history(&conn, &cluster.id, 1)?;
        let last_sync = history.first();

        if json {
            println!(
                "{{\"cluster\": \"{}\", \"drives\": {}, \"last_sync\": {}}}",
                cluster.name,
                drives.len(),
                last_sync
                    .map(|s| format!("\"{}\"", s.finished_at))
                    .unwrap_or_else(|| "null".to_string())
            );
        } else {
            println!("Cluster: {}", cluster.name);
            println!("  Topology: {}", cluster.topology);
            println!("  Conflict: {}", cluster.conflict_strategy);
            println!("  Drives:   {} connected", drives.len());
            for d in &drives {
                let root = d.effective_root();
                let connected = root.exists();
                let sync_info = if d.sync_root.is_some() {
                    format!(" -> {}", root.display())
                } else {
                    String::new()
                };
                println!(
                    "    {} {} ({}){} {}{}",
                    if connected { "+" } else { "-" },
                    d.identity.identity_string(),
                    d.role,
                    if d.is_primary { " [PRIMARY]" } else { "" },
                    d.mount_point.display(),
                    sync_info,
                );
            }
            match last_sync {
                Some(s) => {
                    println!("  Last sync: {} ({})", s.finished_at, s.status);
                    println!(
                        "    {} files, {} bytes transferred",
                        s.files_synced, s.bytes_transferred
                    );
                }
                None => {
                    println!("  Last sync: never");
                }
            }
            println!();
        }
    }

    if clusters.is_empty() && !json {
        println!("No clusters found. Create one with: diffr cluster create <name>");
    }

    Ok(())
}
