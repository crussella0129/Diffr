use std::path::PathBuf;

/// Central error type for the Diffr system.
#[derive(Debug, thiserror::Error)]
pub enum DiffrError {
    #[error("cluster not found: {name}")]
    ClusterNotFound { name: String },

    #[error("cluster already exists: {name}")]
    ClusterAlreadyExists { name: String },

    #[error("drive not found: {identity}")]
    DriveNotFound { identity: String },

    #[error("drive already registered: {identity}")]
    DriveAlreadyRegistered { identity: String },

    #[error("drive not connected: {identity}")]
    DriveNotConnected { identity: String },

    #[error("drive disconnected during sync: {identity}")]
    DriveDisconnected { identity: String },

    #[error("file conflict at {path}")]
    Conflict { path: PathBuf },

    #[error("archive entry not found: {id}")]
    ArchiveNotFound { id: String },

    #[error("path not found: {path}")]
    PathNotFound { path: PathBuf },

    #[error("diffr repo not initialized at {path} (run `diffr init`)")]
    RepoNotInitialized { path: PathBuf },

    #[error("config error: {message}")]
    Config { message: String },

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("{0}")]
    Other(String),
}
