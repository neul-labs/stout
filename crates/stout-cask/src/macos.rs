//! macOS-specific cask installation

use crate::download::ArtifactType;
use crate::error::{Error, Result};
use crate::install::CaskInstallOptions;
use std::path::{Path, PathBuf};
use std::process::Command;
use stout_index::Cask;
use tracing::{debug, info, warn};

/// RAII guard for temporary directory cleanup
struct TempDirGuard(PathBuf);

impl TempDirGuard {
    #[allow(dead_code)]
    fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            if let Err(e) = std::fs::remove_dir_all(&self.0) {
                warn!(
                    "Failed to clean up temp directory {}: {}",
                    self.0.display(),
                    e
                );
            }
        }
    }
}

/// Install artifact on macOS
pub async fn install_artifact(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    match artifact_type {
        ArtifactType::Dmg => install_from_dmg(cask, artifact_path, options).await,
        ArtifactType::Pkg => install_from_pkg(artifact_path, options).await,
        ArtifactType::Zip => install_from_zip(cask, artifact_path, options).await,
        ArtifactType::TarGz | ArtifactType::TarBz2 => {
            install_from_archive(cask, artifact_path, options).await
        }
        ArtifactType::AppImage => Err(Error::UnsupportedPlatform(
            "AppImage is not supported on macOS".to_string(),
        )),
    }
}

/// Install from DMG
async fn install_from_dmg(
    _cask: &Cask,
    dmg_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let mount_point = mount_dmg(dmg_path)?;

    // Find .app bundle in mounted DMG
    let app_bundle = find_app_in_dir(&mount_point)?;

    // Determine destination
    let appdir = options
        .appdir
        .clone()
        .unwrap_or_else(|| PathBuf::from("/Applications"));
    let app_name = app_bundle
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = appdir.join(&app_name);

    // Remove existing if force
    if dest.exists() {
        if options.force {
            info!("Removing existing {}", dest.display());
            std::fs::remove_dir_all(&dest)?;
        } else {
            unmount_dmg(&mount_point)?;
            return Err(Error::InstallFailed(format!(
                "{} already exists. Use --force to overwrite.",
                dest.display()
            )));
        }
    }

    // Copy app to Applications
    info!("Installing {} to {}", app_name, appdir.display());
    copy_dir_all(&app_bundle, &dest)?;

    // Unmount DMG
    unmount_dmg(&mount_point)?;

    // Remove quarantine
    remove_quarantine(&dest)?;

    Ok(dest)
}

/// Install from PKG
async fn install_from_pkg(pkg_path: &Path, _options: &CaskInstallOptions) -> Result<PathBuf> {
    info!("Installing package {}...", pkg_path.display());

    // Security: Verify PKG file exists and is a regular file before running installer
    if !pkg_path.exists() {
        return Err(Error::InstallFailed(format!(
            "PKG file not found: {}",
            pkg_path.display()
        )));
    }
    if !pkg_path.is_file() {
        return Err(Error::InstallFailed(format!(
            "PKG path is not a file: {}",
            pkg_path.display()
        )));
    }

    // PKG installation requires sudo - use installer command
    // Use explicit argument separation to prevent any potential injection
    let output = Command::new("sudo")
        .args(["installer", "-pkg"])
        .arg(pkg_path)
        .arg("--")
        .args(["-target", "/"])
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "installer".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!(
            "Package installation failed: {}",
            stderr
        )));
    }

    // PKG installs to various locations, return a placeholder
    Ok(PathBuf::from("/Applications"))
}

/// Install from ZIP
async fn install_from_zip(
    cask: &Cask,
    zip_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;
    let temp_dir = std::env::temp_dir().join(format!("stout-{}", token));
    std::fs::create_dir_all(&temp_dir)?;

    // Use RAII guard for automatic cleanup
    let _guard = TempDirGuard(temp_dir.clone());

    // Extract ZIP
    let output = Command::new("unzip")
        .args(["-q", "-o"])
        .arg(zip_path)
        .args(["-d"])
        .arg(&temp_dir)
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "unzip".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!("Unzip failed: {}", stderr)));
    }

    // Find .app bundle
    let app_bundle = find_app_in_dir(&temp_dir)?;

    // Determine destination
    let appdir = options
        .appdir
        .clone()
        .unwrap_or_else(|| PathBuf::from("/Applications"));
    let app_name = app_bundle
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = appdir.join(&app_name);

    // Remove existing if force
    if dest.exists() && options.force {
        std::fs::remove_dir_all(&dest)?;
    }

    // Move app to Applications
    info!("Installing {} to {}", app_name, appdir.display());
    copy_dir_all(&app_bundle, &dest)?;

    // Guard will clean up on drop

    // Remove quarantine
    remove_quarantine(&dest)?;

    Ok(dest)
}

/// Install from tar.gz or tar.bz2
async fn install_from_archive(
    cask: &Cask,
    archive_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;
    let temp_dir = std::env::temp_dir().join(format!("stout-{}", token));
    std::fs::create_dir_all(&temp_dir)?;

    // Use RAII guard for automatic cleanup
    let _guard = TempDirGuard(temp_dir.clone());

    // Extract archive
    let output = Command::new("tar")
        .args(["-xf"])
        .arg(archive_path)
        .args(["-C"])
        .arg(&temp_dir)
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "tar".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!("Extract failed: {}", stderr)));
    }

    // Find .app bundle
    let app_bundle = find_app_in_dir(&temp_dir)?;

    // Determine destination
    let appdir = options
        .appdir
        .clone()
        .unwrap_or_else(|| PathBuf::from("/Applications"));
    let app_name = app_bundle
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    let dest = appdir.join(&app_name);

    // Remove existing if force
    if dest.exists() && options.force {
        std::fs::remove_dir_all(&dest)?;
    }

    // Move app to Applications
    copy_dir_all(&app_bundle, &dest)?;

    // Guard will clean up on drop

    // Remove quarantine
    remove_quarantine(&dest)?;

    Ok(dest)
}

/// Mount a DMG file
fn mount_dmg(dmg_path: &Path) -> Result<PathBuf> {
    let mount_point = std::env::temp_dir().join(format!(
        "stout-mount-{}",
        dmg_path.file_stem().unwrap().to_string_lossy()
    ));

    // Create mount point
    std::fs::create_dir_all(&mount_point)?;

    debug!(
        "Mounting {} at {}",
        dmg_path.display(),
        mount_point.display()
    );

    let output = Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
        .arg(&mount_point)
        .arg(dmg_path)
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "hdiutil attach".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::MountFailed(format!(
            "Failed to mount DMG: {}",
            stderr
        )));
    }

    Ok(mount_point)
}

/// Unmount a DMG
fn unmount_dmg(mount_point: &Path) -> Result<()> {
    debug!("Unmounting {}", mount_point.display());

    let output = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(mount_point)
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "hdiutil detach".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("Failed to unmount DMG: {}", stderr);
    }

    Ok(())
}

/// Find a .app bundle in a directory
fn find_app_in_dir(dir: &Path) -> Result<PathBuf> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "app").unwrap_or(false) {
            return Ok(path);
        }

        // Check one level deeper
        if path.is_dir() {
            for subentry in std::fs::read_dir(&path)? {
                let subentry = subentry?;
                let subpath = subentry.path();
                if subpath.extension().map(|e| e == "app").unwrap_or(false) {
                    return Ok(subpath);
                }
            }
        }
    }

    Err(Error::ArtifactNotFound(
        "No .app bundle found in artifact".to_string(),
    ))
}

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Remove quarantine attribute
fn remove_quarantine(path: &Path) -> Result<()> {
    debug!("Removing quarantine from {}", path.display());

    let output = Command::new("xattr")
        .args(["-rd", "com.apple.quarantine"])
        .arg(path)
        .output();

    // Ignore errors - quarantine removal is best-effort
    if let Err(e) = output {
        debug!("xattr command failed (non-fatal): {}", e);
    }

    Ok(())
}
