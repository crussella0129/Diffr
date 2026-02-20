pub mod archive;
pub mod cluster;
pub mod config;
pub mod drive;
pub mod history;
pub mod init;
pub mod status;
pub mod sync;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Command {
    /// Initialize or manage Diffr configuration
    Config {
        #[command(subcommand)]
        action: config::ConfigAction,
    },
    /// Manage clusters
    Cluster {
        #[command(subcommand)]
        action: cluster::ClusterAction,
    },
    /// Manage drives
    Drive {
        #[command(subcommand)]
        action: drive::DriveAction,
    },
    /// Sync a cluster
    Sync(sync::SyncArgs),
    /// Show cluster status
    Status(status::StatusArgs),
    /// Show sync history
    History(history::HistoryArgs),
    /// Initialize a diffr repo at a directory
    Init(init::InitArgs),
    /// Manage archives
    Archive {
        #[command(subcommand)]
        action: archive::ArchiveAction,
    },
}

pub fn run(cmd: Command, json: bool) -> anyhow::Result<()> {
    match cmd {
        Command::Config { action } => config::run(action),
        Command::Cluster { action } => cluster::run(action, json),
        Command::Drive { action } => drive::run(action, json),
        Command::Init(args) => init::run(args),
        Command::Sync(args) => sync::run(args, json),
        Command::Status(args) => status::run(args, json),
        Command::History(args) => history::run(args, json),
        Command::Archive { action } => archive::run(action, json),
    }
}
