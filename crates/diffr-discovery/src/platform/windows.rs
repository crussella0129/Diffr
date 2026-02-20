use diffr_core::models::drive::{Drive, DriveIdentity};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::DriveDiscovery;

pub struct WindowsDiscovery;

impl DriveDiscovery for WindowsDiscovery {
    fn discover_drives(&self) -> anyhow::Result<Vec<Drive>> {
        discover_windows_drives()
    }

    fn find_by_serial(&self, serial: &str) -> anyhow::Result<Option<Drive>> {
        let drives = self.discover_drives()?;
        Ok(drives
            .into_iter()
            .find(|d| d.identity.identity_string() == serial))
    }
}

#[cfg(target_os = "windows")]
fn discover_windows_drives() -> anyhow::Result<Vec<Drive>> {
    use serde::Deserialize;
    use wmi::{COMLibrary, WMIConnection};

    #[derive(Deserialize)]
    #[serde(rename = "Win32_DiskDrive")]
    #[allow(dead_code)]
    struct DiskDrive {
        #[serde(rename = "SerialNumber")]
        serial_number: Option<String>,
        #[serde(rename = "DeviceID")]
        device_id: String,
        #[serde(rename = "Size")]
        size: Option<u64>,
        #[serde(rename = "Model")]
        model: Option<String>,
    }

    #[derive(Deserialize)]
    #[serde(rename = "Win32_DiskDriveToDiskPartition")]
    struct DiskToPartition {
        #[serde(rename = "Antecedent")]
        antecedent: String,
        #[serde(rename = "Dependent")]
        dependent: String,
    }

    #[derive(Deserialize)]
    #[serde(rename = "Win32_LogicalDiskToPartition")]
    struct LogicalToPartition {
        #[serde(rename = "Antecedent")]
        antecedent: String,
        #[serde(rename = "Dependent")]
        dependent: String,
    }

    #[derive(Deserialize)]
    #[serde(rename = "Win32_LogicalDisk")]
    struct LogicalDisk {
        #[serde(rename = "DeviceID")]
        device_id: String,
        #[serde(rename = "Size")]
        size: Option<u64>,
        #[serde(rename = "FreeSpace")]
        free_space: Option<u64>,
        #[serde(rename = "VolumeName")]
        volume_name: Option<String>,
    }

    let com = COMLibrary::new()?;
    let wmi = WMIConnection::new(com)?;

    let disks: Vec<DiskDrive> = wmi.query()?;
    let disk_to_part: Vec<DiskToPartition> = wmi.query()?;
    let logical_to_part: Vec<LogicalToPartition> = wmi.query()?;
    let logicals: Vec<LogicalDisk> = wmi.query()?;

    // Build mapping: partition -> disk device_id
    let mut part_to_disk: HashMap<String, String> = HashMap::new();
    for dtp in &disk_to_part {
        part_to_disk.insert(dtp.dependent.clone(), dtp.antecedent.clone());
    }

    // Build mapping: logical drive letter -> partition
    let mut logical_to_disk: HashMap<String, String> = HashMap::new();
    for ltp in &logical_to_part {
        if let Some(disk_ref) = part_to_disk.get(&ltp.antecedent) {
            logical_to_disk.insert(ltp.dependent.clone(), disk_ref.clone());
        }
    }

    let mut drives = Vec::new();

    for disk in &disks {
        let serial = disk
            .serial_number
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        // Find logical drives mapped to this physical disk
        let mount_points: Vec<&LogicalDisk> = logicals
            .iter()
            .filter(|ld| {
                logical_to_disk
                    .values()
                    .any(|disk_ref| disk_ref.contains(&disk.device_id))
                    && logical_to_disk
                        .keys()
                        .any(|k| k.contains(&ld.device_id))
            })
            .collect();

        for logical in mount_points {
            let mount = PathBuf::from(format!("{}\\", logical.device_id));
            let identity = match &serial {
                Some(s) => DriveIdentity::new_hardware(s.clone()),
                None => {
                    // Try to read synthetic ID from drive, or create one
                    crate::read_or_create_synthetic_id(&mount)
                        .unwrap_or_else(|_| DriveIdentity::new_synthetic())
                }
            };

            let mut drive = Drive::new(identity, mount);
            drive.label = logical
                .volume_name
                .clone()
                .or_else(|| disk.model.clone());
            drive.total_bytes = logical.size;
            drive.free_bytes = logical.free_space;
            drives.push(drive);
        }
    }

    Ok(drives)
}

#[cfg(not(target_os = "windows"))]
fn discover_windows_drives() -> anyhow::Result<Vec<Drive>> {
    Ok(Vec::new())
}
