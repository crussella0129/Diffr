pub mod platform;

use diffr_core::models::drive::{Drive, DriveIdentity};
use std::path::Path;

/// Trait for platform-specific drive discovery.
pub trait DriveDiscovery {
    /// Discover all connected drives.
    fn discover_drives(&self) -> anyhow::Result<Vec<Drive>>;

    /// Find a specific drive by its serial number.
    fn find_by_serial(&self, serial: &str) -> anyhow::Result<Option<Drive>>;
}

/// Read or create a synthetic drive identity file on the drive.
pub fn read_or_create_synthetic_id(drive_root: &Path) -> anyhow::Result<DriveIdentity> {
    let diffr_dir = drive_root.join(".diffr");
    let identity_path = diffr_dir.join("drive_identity.toml");

    if identity_path.exists() {
        let content = std::fs::read_to_string(&identity_path)?;
        let identity: DriveIdentity = toml::from_str(&content)?;
        Ok(identity)
    } else {
        std::fs::create_dir_all(&diffr_dir)?;
        let identity = DriveIdentity::new_synthetic();
        let content = toml::to_string_pretty(&identity)?;
        std::fs::write(&identity_path, content)?;
        Ok(identity)
    }
}
