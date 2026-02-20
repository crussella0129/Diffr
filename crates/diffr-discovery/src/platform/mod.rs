#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

use crate::DriveDiscovery;

/// Get the platform-appropriate drive discovery implementation.
pub fn get_discovery() -> Box<dyn DriveDiscovery> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsDiscovery)
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxDiscovery)
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsDiscovery)
    }
}
