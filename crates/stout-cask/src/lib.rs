//! stout-cask: Cask installation and management for stout
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

pub use download::{detect_artifact_type_from_magic, download_cask_artifact, ArtifactType};
pub use error::{Error, Result};
pub use install::{
    install_artifact_only, install_artifact_sync, install_cask, uninstall_cask, CaskInstallOptions,
};
pub use state::{now_timestamp, InstalledCask, InstalledCasks};
use stout_index::Cask;

/// Detect the artifact type from a URL or path.
///
/// First tries the path component of the URL (before `?`); if that has no
/// recognisable extension it searches the whole URL string to catch patterns
/// like `rotation.php?file=HandBrake.dmg&update=true`.
pub fn detect_artifact_type(url: &str) -> ArtifactType {
    // 1. Check path portion only (before query string / fragment)
    let path = url.split('?').next().unwrap_or(url);
    let path = path.split('#').next().unwrap_or(path);
    let path_lower = path.to_lowercase();

    if path_lower.ends_with(".dmg") {
        return ArtifactType::Dmg;
    } else if path_lower.ends_with(".pkg") {
        return ArtifactType::Pkg;
    } else if path_lower.ends_with(".zip") {
        return ArtifactType::Zip;
    } else if path_lower.ends_with(".tar.gz") || path_lower.ends_with(".tgz") {
        return ArtifactType::TarGz;
    } else if path_lower.ends_with(".tar.bz2") || path_lower.ends_with(".tbz2") {
        return ArtifactType::TarBz2;
    } else if path_lower.ends_with(".appimage") {
        return ArtifactType::AppImage;
    }

    // 2. Search the full URL string (catches extension in query params)
    let url_lower = url.to_lowercase();
    if url_lower.contains(".dmg") {
        ArtifactType::Dmg
    } else if url_lower.contains(".pkg") {
        ArtifactType::Pkg
    } else if url_lower.contains(".zip") {
        ArtifactType::Zip
    } else if url_lower.contains(".tar.gz") || url_lower.contains(".tgz") {
        ArtifactType::TarGz
    } else if url_lower.contains(".tar.bz2") || url_lower.contains(".tbz2") {
        ArtifactType::TarBz2
    } else if url_lower.contains(".appimage") {
        ArtifactType::AppImage
    } else {
        // No extension found anywhere — caller should use magic-byte detection
        // after downloading. Return Zip as a placeholder.
        ArtifactType::Zip
    }
}

/// Detect artifact type from cask metadata (preferred) or URL fallback
pub fn detect_artifact_type_from_cask(cask: &Cask, url: &str) -> ArtifactType {
    // Check cask container type first (most reliable)
    if let Some(ref container) = cask.container {
        if let Some(ref container_type) = container.container_type {
            match container_type.to_lowercase().as_str() {
                "dmg" => return ArtifactType::Dmg,
                "pkg" => return ArtifactType::Pkg,
                "zip" => return ArtifactType::Zip,
                "tar" | "tar_gz" | "tgz" => return ArtifactType::TarGz,
                "tar_bz2" | "tbz2" => return ArtifactType::TarBz2,
                "bz2" | "bzip2" => return ArtifactType::TarBz2,
                "7z" | "seven_zip" => {
                    // Treat 7z as tar for extraction purposes
                    return ArtifactType::TarGz;
                }
                "naked" => {
                    // Naked container - the file itself is the artifact (no extraction)
                    // This is typically for binaries, check URL for actual format
                }
                _ => {}
            }
        }
    }

    // Check primary artifact type - PKG installs are special
    let primary_type = cask.primary_artifact_type();
    if primary_type == "pkg" {
        return ArtifactType::Pkg;
    }

    // Fallback to URL-based detection
    detect_artifact_type(url)
}

/// Get the file extension to use when downloading based on container type
pub fn get_download_extension(cask: &Cask, url: &str) -> &'static str {
    // Check cask container type first
    if let Some(ref container) = cask.container {
        if let Some(ref container_type) = container.container_type {
            match container_type.to_lowercase().as_str() {
                "dmg" => return "dmg",
                "pkg" => return "pkg",
                "zip" => return "zip",
                "tar" | "tar_gz" | "tgz" => return "tar.gz",
                "tar_bz2" | "tbz2" | "bz2" | "bzip2" => return "tar.bz2",
                "7z" | "seven_zip" => return "7z",
                "naked" => {
                    // For naked containers, extract from URL
                }
                _ => {}
            }
        }
    }

    // Check primary artifact type
    if cask.primary_artifact_type() == "pkg" {
        return "pkg";
    }

    // Fallback to URL extension
    let url_lower = url.to_lowercase();
    if url_lower.ends_with(".dmg") {
        "dmg"
    } else if url_lower.ends_with(".pkg") {
        "pkg"
    } else if url_lower.ends_with(".zip") {
        "zip"
    } else if url_lower.ends_with(".tar.gz") || url_lower.ends_with(".tgz") {
        "tar.gz"
    } else if url_lower.ends_with(".tar.bz2") || url_lower.ends_with(".tbz2") {
        "tar.bz2"
    } else {
        "zip" // Default
    }
}
