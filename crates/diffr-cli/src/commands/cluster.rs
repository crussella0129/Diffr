use clap::Subcommand;
use diffr_core::config::DiffrConfig;
use diffr_core::models::cluster::{Cluster, ConflictStrategy, Topology};
use diffr_db::ops;

#[derive(Subcommand)]
pub enum ClusterAction {
    /// Create a new cluster
    Create {
        /// Cluster name
        name: String,
        /// Sync topology: mesh or primary-replica
        #[arg(long, default_value = "mesh")]
        topology: String,
        /// Conflict strategy: newest-wins, keep-both, or interactive
        #[arg(long, default_value = "newest-wins")]
        conflict: String,
    },
    /// List all clusters
    List,
    /// Show detailed cluster info
    Info {
        /// Cluster name
        name: String,
    },
    /// Remove a cluster
    Remove {
        /// Cluster name
        name: String,
    },
}

pub fn run(action: ClusterAction, json: bool) -> anyhow::Result<()> {
    let db_path = DiffrConfig::db_path()?;
    let conn = diffr_db::open_db(&db_path)?;

    match action {
        ClusterAction::Create {
            name,
            topology,
            conflict,
        } => {
            let topo: Topology = topology
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;
            let strategy: ConflictStrategy = conflict
                .parse()
                .map_err(|e: String| anyhow::anyhow!(e))?;

            // Check if cluster already exists
            if ops::get_cluster_by_name(&conn, &name)?.is_some() {
                anyhow::bail!("cluster '{}' already exists", name);
            }

            let cluster = Cluster::new(name.clone(), topo, strategy);
            ops::insert_cluster(&conn, &cluster)?;

            if json {
                println!(
                    "{{\"id\": \"{}\", \"name\": \"{}\"}}",
                    cluster.id, cluster.name
                );
            } else {
                println!("Created cluster '{}' ({})", cluster.name, cluster.id);
            }
            Ok(())
        }
        ClusterAction::List => {
            let clusters = ops::list_clusters(&conn)?;
            if json {
                let items: Vec<_> = clusters
                    .iter()
                    .map(|c| {
                        format!(
                            "{{\"id\": \"{}\", \"name\": \"{}\", \"topology\": \"{}\", \"conflict_strategy\": \"{}\"}}",
                            c.id, c.name, c.topology, c.conflict_strategy
                        )
                    })
                    .collect();
                println!("[{}]", items.join(", "));
            } else {
                if clusters.is_empty() {
                    println!("No clusters found. Create one with: diffr cluster create <name>");
                } else {
                    println!("{:<40} {:<15} {:<15}", "NAME", "TOPOLOGY", "CONFLICT");
                    for c in &clusters {
                        println!("{:<40} {:<15} {:<15}", c.name, c.topology, c.conflict_strategy);
                    }
                }
            }
            Ok(())
        }
        ClusterAction::Info { name } => {
            let cluster = ops::get_cluster_by_name(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", name))?;
            let drives = ops::list_drives_for_cluster(&conn, &cluster.id)?;

            if json {
                println!(
                    "{{\"id\": \"{}\", \"name\": \"{}\", \"topology\": \"{}\", \"conflict_strategy\": \"{}\", \"drives\": {}}}",
                    cluster.id, cluster.name, cluster.topology, cluster.conflict_strategy, drives.len()
                );
            } else {
                println!("Cluster: {}", cluster.name);
                println!("  ID:       {}", cluster.id);
                println!("  Topology: {}", cluster.topology);
                println!("  Conflict: {}", cluster.conflict_strategy);
                println!("  Created:  {}", cluster.created_at);
                println!("  Drives:   {}", drives.len());
                for d in &drives {
                    println!(
                        "    - {} ({}) at {}{}",
                        d.identity.identity_string(),
                        d.role,
                        d.mount_point.display(),
                        if d.is_primary { " [PRIMARY]" } else { "" }
                    );
                }
            }
            Ok(())
        }
        ClusterAction::Remove { name } => {
            let cluster = ops::get_cluster_by_name(&conn, &name)?
                .ok_or_else(|| anyhow::anyhow!("cluster '{}' not found", name))?;
            ops::delete_cluster(&conn, &cluster.id)?;
            println!("Removed cluster '{}'", name);
            Ok(())
        }
    }
}
