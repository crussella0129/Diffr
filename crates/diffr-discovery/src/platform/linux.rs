use diffr_core::models::drive::{Drive, DriveIdentity};
use std::path::PathBuf;
use std::process::Command;

use crate::DriveDiscovery;

pub struct LinuxDiscovery;

impl DriveDiscovery for LinuxDiscovery {
    fn discover_drives(&self) -> anyhow::Result<Vec<Drive>> {
        discover_linux_drives()
    }

    fn find_by_serial(&self, serial: &str) -> anyhow::Result<Option<Drive>> {
        let drives = self.discover_drives()?;
        Ok(drives
            .into_iter()
            .find(|d| d.identity.identity_string() == serial))
    }
}

#[cfg(target_os = "linux")]
fn discover_linux_drives() -> anyhow::Result<Vec<Drive>> {
    use serde::Deserialize;

    #[derive(Deserialize)]
    struct LsblkOutput {
        blockdevices: Vec<BlockDevice>,
    }

    #[derive(Deserialize)]
    struct BlockDevice {
        name: String,
        serial: Option<String>,
        size: Option<String>,
        mountpoint: Option<String>,
        label: Option<String>,
        #[serde(rename = "type")]
        device_type: Option<String>,
        children: Option<Vec<BlockDevice>>,
    }

    let output = Command::new("lsblk")
        .args(["--json", "-o", "NAME,SERIAL,SIZE,MOUNTPOINT,LABEL,TYPE"])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("lsblk failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    let lsblk: LsblkOutput = serde_json::from_slice(&output.stdout)?;
    let mut drives = Vec::new();

    for device in &lsblk.blockdevices {
        let serial = device
            .serial
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // Check children (partitions) for mount points
        if let Some(children) = &device.children {
            for child in children {
                if let Some(ref mountpoint) = child.mountpoint {
                    if mountpoint == "[SWAP]" {
                        continue;
                    }
                    let mount = PathBuf::from(mountpoint);
                    let identity = match &serial {
                        Some(s) => DriveIdentity::new_hardware(s.clone()),
                        None => crate::read_or_create_synthetic_id(&mount)
                            .unwrap_or_else(|_| DriveIdentity::new_synthetic()),
                    };
                    let mut drive = Drive::new(identity, mount);
                    drive.label = child.label.clone().or_else(|| Some(device.name.clone()));
                    drives.push(drive);
                }
            }
        }

        // Also check the device itself if it has a mount point (e.g. whole-disk filesystem)
        if let Some(ref mountpoint) = device.mountpoint {
            if mountpoint != "[SWAP]" {
                let mount = PathBuf::from(mountpoint);
                let identity = match &serial {
                    Some(s) => DriveIdentity::new_hardware(s.clone()),
                    None => crate::read_or_create_synthetic_id(&mount)
                        .unwrap_or_else(|_| DriveIdentity::new_synthetic()),
                };
                let mut drive = Drive::new(identity, mount);
                drive.label = device.label.clone();
                drives.push(drive);
            }
        }
    }

    Ok(drives)
}

#[cfg(not(target_os = "linux"))]
fn discover_linux_drives() -> anyhow::Result<Vec<Drive>> {
    Ok(Vec::new())
}
