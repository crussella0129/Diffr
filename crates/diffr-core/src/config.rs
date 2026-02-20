use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::error::DiffrError;
use crate::models::archive::RetentionPolicy;
use crate::models::cluster::{ConflictStrategy, Topology};

/// Top-level Diffr configuration, stored at `~/.diffr/config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffrConfig {
    /// Default topology for new clusters.
    #[serde(default = "default_topology")]
    pub default_topology: Topology,

    /// Default conflict strategy for new clusters.
    #[serde(default = "default_conflict_strategy")]
    pub default_conflict_strategy: ConflictStrategy,

    /// Default retention policy for archives.
    #[serde(default)]
    pub retention: RetentionPolicy,

    /// Whether to enable content hashing by default (vs metadata-only).
    #[serde(default)]
    pub hash_by_default: bool,

    /// Whether to verify with SHA-256 after sync.
    #[serde(default)]
    pub verify_after_sync: bool,
}

fn default_topology() -> Topology {
    Topology::Mesh
}

fn default_conflict_strategy() -> ConflictStrategy {
    ConflictStrategy::NewestWins
}

impl Default for DiffrConfig {
    fn default() -> Self {
        Self {
            default_topology: Topology::Mesh,
            default_conflict_strategy: ConflictStrategy::NewestWins,
            retention: RetentionPolicy::default(),
            hash_by_default: false,
            verify_after_sync: false,
        }
    }
}

impl DiffrConfig {
    /// Returns the Diffr home directory (`~/.diffr/`).
    pub fn home_dir() -> Result<PathBuf, DiffrError> {
        let base = dirs::home_dir().ok_or_else(|| DiffrError::Config {
            message: "could not determine home directory".into(),
        })?;
        Ok(base.join(".diffr"))
    }

    /// Returns the path to the config file.
    pub fn config_path() -> Result<PathBuf, DiffrError> {
        Ok(Self::home_dir()?.join("config.toml"))
    }

    /// Returns the path to the database file.
    pub fn db_path() -> Result<PathBuf, DiffrError> {
        Ok(Self::home_dir()?.join("diffr.db"))
    }

    /// Load config from the default location, or return defaults if not found.
    pub fn load() -> Result<Self, DiffrError> {
        let path = Self::config_path()?;
        if path.exists() {
            Self::load_from(&path)
        } else {
            Ok(Self::default())
        }
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Result<Self, DiffrError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| DiffrError::Serialization(e.to_string()))
    }

    /// Save config to the default location.
    pub fn save(&self) -> Result<(), DiffrError> {
        let path = Self::config_path()?;
        self.save_to(&path)
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), DiffrError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| DiffrError::Serialization(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Initialize the Diffr home directory with default config.
    pub fn init() -> Result<PathBuf, DiffrError> {
        let home = Self::home_dir()?;
        std::fs::create_dir_all(&home)?;

        let config_path = Self::config_path()?;
        if !config_path.exists() {
            Self::default().save_to(&config_path)?;
        }

        Ok(home)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_roundtrip() {
        let config = DiffrConfig::default();
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: DiffrConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            config.default_topology,
            deserialized.default_topology
        );
    }
}
