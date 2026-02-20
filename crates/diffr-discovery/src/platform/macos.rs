use diffr_core::models::drive::{Drive, DriveIdentity};
use std::path::PathBuf;
use std::process::Command;

use crate::DriveDiscovery;

pub struct MacOsDiscovery;

impl DriveDiscovery for MacOsDiscovery {
    fn discover_drives(&self) -> anyhow::Result<Vec<Drive>> {
        discover_macos_drives()
    }

    fn find_by_serial(&self, serial: &str) -> anyhow::Result<Option<Drive>> {
        let drives = self.discover_drives()?;
        Ok(drives
            .into_iter()
            .find(|d| d.identity.identity_string() == serial))
    }
}

#[cfg(target_os = "macos")]
fn discover_macos_drives() -> anyhow::Result<Vec<Drive>> {
    // List all disks via diskutil
    let output = Command::new("diskutil").args(["list", "-plist"]).output()?;

    if !output.status.success() {
        anyhow::bail!(
            "diskutil list failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let plist: plist::Value = plist::from_bytes(&output.stdout)?;
    let mut drives = Vec::new();

    // Get all disk identifiers
    if let Some(all_disks) = plist
        .as_dictionary()
        .and_then(|d| d.get("AllDisks"))
        .and_then(|v| v.as_array())
    {
        for disk_val in all_disks {
            if let Some(disk_id) = disk_val.as_string() {
                // Skip synthesized/virtual disks
                if disk_id.starts_with("synthesized") {
                    continue;
                }
                // Get info for this disk
                if let Ok(info) = get_disk_info(disk_id) {
                    if let Some(mount_point) = info.mount_point {
                        let identity = match info.serial {
                            Some(s) if !s.is_empty() => DriveIdentity::new_hardware(s),
                            _ => {
                                let mount = PathBuf::from(&mount_point);
                                crate::read_or_create_synthetic_id(&mount)
                                    .unwrap_or_else(|_| DriveIdentity::new_synthetic())
                            }
                        };
                        let mut drive = Drive::new(identity, PathBuf::from(&mount_point));
                        drive.label = info.volume_name;
                        drive.total_bytes = info.total_size;
                        drive.free_bytes = info.free_space;
                        drives.push(drive);
                    }
                }
            }
        }
    }

    Ok(drives)
}

#[cfg(target_os = "macos")]
struct DiskInfo {
    serial: Option<String>,
    mount_point: Option<String>,
    volume_name: Option<String>,
    total_size: Option<u64>,
    free_space: Option<u64>,
}

#[cfg(target_os = "macos")]
fn get_disk_info(disk_id: &str) -> anyhow::Result<DiskInfo> {
    let output = Command::new("diskutil")
        .args(["info", "-plist", disk_id])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("diskutil info failed for {}", disk_id);
    }

    let plist: plist::Value = plist::from_bytes(&output.stdout)?;
    let dict = plist
        .as_dictionary()
        .ok_or_else(|| anyhow::anyhow!("expected dictionary"))?;

    Ok(DiskInfo {
        serial: dict
            .get("IORegistryEntrySerialNumber")
            .and_then(|v| v.as_string())
            .map(|s| s.trim().to_string()),
        mount_point: dict
            .get("MountPoint")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty()),
        volume_name: dict
            .get("VolumeName")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string()),
        total_size: dict
            .get("TotalSize")
            .and_then(|v| v.as_unsigned_integer()),
        free_space: dict
            .get("APFSContainerFree")
            .or_else(|| dict.get("FreeSpace"))
            .and_then(|v| v.as_unsigned_integer()),
    })
}

#[cfg(not(target_os = "macos"))]
fn discover_macos_drives() -> anyhow::Result<Vec<Drive>> {
    Ok(Vec::new())
}
