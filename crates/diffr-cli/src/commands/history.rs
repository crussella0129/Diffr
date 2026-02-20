use clap::Args;
use diffr_core::config::DiffrConfig;
use diffr_db::ops;

#[derive(Args)]
pub struct HistoryArgs {
    /// Cluster name
    cluster: String,

    /// Maximum number of entries to show
    #[arg(long, default_value = "20")]
    limit: u32,
}

pub fn run(args: HistoryArgs, json: bool) -> anyhow::Result<()> {
    let db_path = DiffrConfig::db_path()?;
    let conn = diffr_db::open_db(&db_path)?;

    let cluster = ops::get_cluster_by_name(&conn, &args.cluster)?
        .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", args.cluster))?;

    let history = ops::list_sync_history(&conn, &cluster.id, args.limit)?;

    if json {
        let items: Vec<_> = history
            .iter()
            .map(|s| {
                format!(
                    "{{\"id\": \"{}\", \"started\": \"{}\", \"finished\": \"{}\", \"status\": \"{}\", \"files\": {}, \"bytes\": {}}}",
                    s.id, s.started_at, s.finished_at, s.status, s.files_synced, s.bytes_transferred
                )
            })
            .collect();
        println!("[{}]", items.join(", "));
    } else {
        if history.is_empty() {
            println!("No sync history for cluster '{}'", cluster.name);
        } else {
            println!(
                "{:<24} {:<16} {:>8} {:>12} {:>8}",
                "FINISHED", "STATUS", "FILES", "BYTES", "ERRORS"
            );
            for s in &history {
                println!(
                    "{:<24} {:<16} {:>8} {:>12} {:>8}",
                    s.finished_at.format("%Y-%m-%d %H:%M:%S"),
                    s.status,
                    s.files_synced,
                    s.bytes_transferred,
                    s.errors.len()
                );
            }
        }
    }

    Ok(())
}
