//! Linux-specific cask installation (AppImage, Flatpak)

use crate::download::ArtifactType;
use crate::error::{Error, Result};
use crate::install::CaskInstallOptions;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use stout_index::Cask;
use tracing::{debug, info, warn};

/// Validate a cask token for safe use in file paths
fn validate_token(token: &str) -> Result<()> {
    if token.is_empty() {
        return Err(Error::InvalidInput(
            "cask token cannot be empty".to_string(),
        ));
    }
    if token.contains("..") || token.contains('/') || token.contains('\0') {
        return Err(Error::InvalidInput(format!(
            "cask token '{}' contains invalid characters for file path",
            token
        )));
    }
    Ok(())
}

/// RAII guard for temporary directory cleanup
struct TempDirGuard(PathBuf);

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

/// Install artifact on Linux
pub async fn install_artifact(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    // Delegate to sync version
    install_artifact_sync(cask, artifact_path, artifact_type, options)
}

/// Synchronous version for use with spawn_blocking
pub fn install_artifact_sync(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    match artifact_type {
        ArtifactType::AppImage => install_appimage_sync(cask, artifact_path, options),
        ArtifactType::Zip | ArtifactType::TarGz | ArtifactType::TarBz2 => {
            install_from_archive_sync(cask, artifact_path, artifact_type, options)
        }
        ArtifactType::Dmg | ArtifactType::Pkg => Err(Error::UnsupportedPlatform(
            "DMG and PKG are macOS-only formats".to_string(),
        )),
    }
}

/// Install an AppImage (synchronous)
fn install_appimage_sync(
    cask: &Cask,
    appimage_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;

    // Determine install location
    let appimage_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("stout")
        .join("appimages");
    std::fs::create_dir_all(&appimage_dir)?;

    let dest = appimage_dir.join(format!("{}.AppImage", token));

    // Copy AppImage
    if dest.exists() && options.force {
        std::fs::remove_file(&dest)?;
    }

    info!("Installing {} to {}", token, dest.display());
    std::fs::copy(appimage_path, &dest)?;

    // Make executable
    let mut perms = std::fs::metadata(&dest)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&dest, perms)?;

    // Create symlink in ~/.local/bin
    let bin_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".local")
        .join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    let bin_link = bin_dir.join(token);
    if bin_link.exists() || bin_link.is_symlink() {
        std::fs::remove_file(&bin_link)?;
    }
    std::os::unix::fs::symlink(&dest, &bin_link)?;

    info!(
        "Created symlink {} -> {}",
        bin_link.display(),
        dest.display()
    );

    // Try to extract .desktop file
    if let Err(e) = extract_desktop_file(&dest, token) {
        debug!("Could not extract desktop file: {}", e);
    }

    Ok(dest)
}

/// Install from archive on Linux (synchronous)
fn install_from_archive_sync(
    cask: &Cask,
    archive_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;

    // Validate token to prevent path traversal
    validate_token(token)?;

    let temp_dir = std::env::temp_dir().join(format!("stout-{}", token));
    std::fs::create_dir_all(&temp_dir)?;

    // Use RAII guard for automatic cleanup
    let _guard = TempDirGuard(temp_dir.clone());

    // Extract archive
    let extract_args: Vec<&str> = match artifact_type {
        ArtifactType::Zip => vec!["-xf"],
        ArtifactType::TarGz => vec!["-xzf"],
        ArtifactType::TarBz2 => vec!["-xjf"],
        _ => {
            return Err(Error::UnsupportedPlatform(
                "Unknown archive type".to_string(),
            ))
        }
    };

    let cmd = if artifact_type == ArtifactType::Zip {
        "unzip"
    } else {
        "tar"
    };

    let output = if artifact_type == ArtifactType::Zip {
        Command::new("unzip")
            .args(["-q", "-o"])
            .arg(archive_path)
            .args(["-d"])
            .arg(&temp_dir)
            .output()
    } else {
        Command::new("tar")
            .args(&extract_args)
            .arg(archive_path)
            .args(["-C"])
            .arg(&temp_dir)
            .output()
    }
    .map_err(|e| Error::CommandFailed {
        cmd: cmd.to_string(),
        message: e.to_string(),
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!("Extract failed: {}", stderr)));
    }

    // Look for AppImage or executable
    if let Some(appimage) = find_file_by_extension(&temp_dir, "AppImage")? {
        let result = install_appimage_sync(cask, &appimage, options)?;
        return Ok(result);
    }

    // Look for executable binary
    if let Some(binary) = find_executable(&temp_dir, token)? {
        let bin_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".local")
            .join("bin");
        std::fs::create_dir_all(&bin_dir)?;

        let dest = bin_dir.join(token);
        if dest.exists() && options.force {
            std::fs::remove_file(&dest)?;
        }

        std::fs::copy(&binary, &dest)?;

        // Make executable
        let mut perms = std::fs::metadata(&dest)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest, perms)?;

        return Ok(dest);
    }

    Err(Error::ArtifactNotFound(
        "No AppImage or executable found in archive".to_string(),
    ))
}

/// Find a file by extension in a directory (recursive)
fn find_file_by_extension(dir: &Path, ext: &str) -> Result<Option<PathBuf>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(file_ext) = path.extension() {
                if file_ext == ext {
                    return Ok(Some(path));
                }
            }
            // Also check if filename ends with extension (e.g., "foo.AppImage")
            if path.to_string_lossy().ends_with(&format!(".{}", ext)) {
                return Ok(Some(path));
            }
        } else if path.is_dir() {
            if let Some(found) = find_file_by_extension(&path, ext)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

/// Find an executable in a directory
fn find_executable(dir: &Path, preferred_name: &str) -> Result<Option<PathBuf>> {
    // First try to find one with the preferred name
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let name = path.file_name().unwrap().to_string_lossy();
            if name.to_lowercase() == preferred_name.to_lowercase() && is_executable(&path) {
                return Ok(Some(path));
            }
        } else if path.is_dir() {
            if let Some(found) = find_executable(&path, preferred_name)? {
                return Ok(Some(found));
            }
        }
    }

    // Fall back to any executable
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && is_executable(&path) {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

/// Check if a file is executable
fn is_executable(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::metadata(path) {
        let mode = metadata.permissions().mode();
        mode & 0o111 != 0
    } else {
        false
    }
}

/// Extract .desktop file from AppImage
fn extract_desktop_file(appimage_path: &Path, token: &str) -> Result<()> {
    // Validate token to prevent path traversal
    validate_token(token)?;

    let applications_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("~/.local/share"))
        .join("applications");
    std::fs::create_dir_all(&applications_dir)?;

    // Try to extract using --appimage-extract (works with most AppImages)
    let temp_dir = std::env::temp_dir().join(format!("stout-desktop-{}", token));

    // Security: Verify AppImage is a regular executable file before executing
    if !appimage_path.is_file() {
        warn!("AppImage path is not a file: {}", appimage_path.display());
        return Err(Error::InstallFailed(format!(
            "AppImage is not a file: {}",
            appimage_path.display()
        )));
    }

    let output = Command::new(appimage_path)
        .args(["--appimage-extract", "*.desktop"])
        .current_dir(std::env::temp_dir())
        .env("APPDIR", &temp_dir)
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            // Look for .desktop file in squashfs-root
            let squashfs_root = std::env::temp_dir().join("squashfs-root");
            if let Some(desktop) = find_file_by_extension(&squashfs_root, "desktop")? {
                let dest = applications_dir.join(format!("{}.desktop", token));
                std::fs::copy(&desktop, &dest)?;
                info!("Installed desktop file to {}", dest.display());
            }
            let _ = std::fs::remove_dir_all(&squashfs_root);
        }
    }

    Ok(())
}

/// Install via Flatpak (if available)
#[allow(dead_code)]
pub async fn install_flatpak(app_id: &str, remote: &str) -> Result<PathBuf> {
    info!("Installing {} via Flatpak...", app_id);

    let output = Command::new("flatpak")
        .args(["install", "--user", "-y", remote, app_id])
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "flatpak install".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!(
            "Flatpak install failed: {}",
            stderr
        )));
    }

    // Flatpak manages its own paths
    Ok(PathBuf::from(format!("/var/lib/flatpak/app/{}", app_id)))
}

/// Uninstall a Flatpak app
#[allow(dead_code)]
pub async fn uninstall_flatpak(app_id: &str) -> Result<()> {
    info!("Uninstalling {} via Flatpak...", app_id);

    let output = Command::new("flatpak")
        .args(["uninstall", "--user", "-y", app_id])
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "flatpak uninstall".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::UninstallFailed(format!(
            "Flatpak uninstall failed: {}",
            stderr
        )));
    }

    Ok(())
}
