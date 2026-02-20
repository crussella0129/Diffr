use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a cluster.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterId(pub Uuid);

impl ClusterId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }
}

impl std::fmt::Display for ClusterId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Sync topology for a cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topology {
    /// All drives are equal peers; changes flow in all directions.
    Mesh,
    /// One drive is the primary source of truth; others are replicas.
    PrimaryReplica,
}

impl std::fmt::Display for Topology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Topology::Mesh => write!(f, "mesh"),
            Topology::PrimaryReplica => write!(f, "primary_replica"),
        }
    }
}

impl std::str::FromStr for Topology {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "mesh" => Ok(Topology::Mesh),
            "primary_replica" | "primary-replica" => Ok(Topology::PrimaryReplica),
            _ => Err(format!("unknown topology: {s}")),
        }
    }
}

/// Strategy for resolving file conflicts during sync.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStrategy {
    /// The file with the newest modification time wins.
    NewestWins,
    /// Keep both versions, renaming the conflicting file.
    KeepBoth,
    /// Prompt the user to decide interactively.
    Interactive,
}

impl std::fmt::Display for ConflictStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConflictStrategy::NewestWins => write!(f, "newest_wins"),
            ConflictStrategy::KeepBoth => write!(f, "keep_both"),
            ConflictStrategy::Interactive => write!(f, "interactive"),
        }
    }
}

impl std::str::FromStr for ConflictStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "newest_wins" | "newest-wins" => Ok(ConflictStrategy::NewestWins),
            "keep_both" | "keep-both" => Ok(ConflictStrategy::KeepBoth),
            "interactive" => Ok(ConflictStrategy::Interactive),
            _ => Err(format!("unknown conflict strategy: {s}")),
        }
    }
}

/// A cluster groups drives that sync together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: ClusterId,
    pub name: String,
    pub topology: Topology,
    pub conflict_strategy: ConflictStrategy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Cluster {
    pub fn new(name: String, topology: Topology, conflict_strategy: ConflictStrategy) -> Self {
        let now = Utc::now();
        Self {
            id: ClusterId::new(),
            name,
            topology,
            conflict_strategy,
            created_at: now,
            updated_at: now,
        }
    }
}
