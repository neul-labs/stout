//! brewx-cask: Cask installation and management for brewx
//!
//! This crate handles:
//! - Cask artifact downloading (DMG, PKG, ZIP)
//! - macOS app installation (mount DMG, copy .app)
//! - PKG installer execution
//! - Quarantine attribute removal
//! - Cask state tracking
//! - Linux app support (AppImage, Flatpak)

mod download;
mod error;
mod install;
mod state;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "linux")]
mod linux;

pub use download::{download_cask_artifact, ArtifactType};
pub use error::{Error, Result};
pub use install::{install_cask, uninstall_cask, CaskInstallOptions};
pub use state::{InstalledCask, InstalledCasks};

/// Detect the artifact type from a URL or path
pub fn detect_artifact_type(url: &str) -> ArtifactType {
    let url_lower = url.to_lowercase();

    if url_lower.ends_with(".dmg") {
        ArtifactType::Dmg
    } else if url_lower.ends_with(".pkg") {
        ArtifactType::Pkg
    } else if url_lower.ends_with(".zip") {
        ArtifactType::Zip
    } else if url_lower.ends_with(".tar.gz") || url_lower.ends_with(".tgz") {
        ArtifactType::TarGz
    } else if url_lower.ends_with(".tar.bz2") || url_lower.ends_with(".tbz2") {
        ArtifactType::TarBz2
    } else if url_lower.ends_with(".appimage") {
        ArtifactType::AppImage
    } else {
        // Default to ZIP for unknown
        ArtifactType::Zip
    }
}
