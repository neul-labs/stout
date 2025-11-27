//! Cask installation logic

use crate::download::{download_cask_artifact, ArtifactType};
use crate::error::{Error, Result};
use crate::state::{now_timestamp, InstalledArtifact, InstalledCask, InstalledCasks};
use crate::detect_artifact_type;
use brewx_index::Cask;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Options for cask installation
#[derive(Debug, Clone, Default)]
pub struct CaskInstallOptions {
    /// Force reinstall even if already installed
    pub force: bool,
    /// Skip checksum verification
    pub no_verify: bool,
    /// Custom application directory (default: /Applications)
    pub appdir: Option<PathBuf>,
    /// Dry run - don't actually install
    pub dry_run: bool,
}

/// Install a cask
pub async fn install_cask(
    cask: &Cask,
    cache_dir: &Path,
    state_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;

    // Check if already installed
    let mut installed_casks = InstalledCasks::load(state_path)?;
    if installed_casks.is_installed(token) && !options.force {
        return Err(Error::InstallFailed(format!(
            "{} is already installed. Use --force to reinstall.",
            token
        )));
    }

    // Get download URL
    let url = cask.download_url().ok_or_else(|| {
        Error::InstallFailed(format!("No download URL for cask {}", token))
    })?;

    // Detect artifact type
    let artifact_type = detect_artifact_type(url);

    // Get expected checksum
    let sha256 = if options.no_verify {
        None
    } else {
        cask.sha256.as_str()
    };

    info!("Downloading {}...", token);

    if options.dry_run {
        info!("[dry-run] Would download {} from {}", token, url);
        info!("[dry-run] Would install to /Applications");
        return Ok(PathBuf::from("/Applications"));
    }

    // Download artifact
    let artifact_path = download_cask_artifact(url, cache_dir, token, sha256, artifact_type).await?;

    // Install based on platform and artifact type
    let install_result = install_artifact(cask, &artifact_path, artifact_type, options).await?;

    // Record installation
    let installed = InstalledCask {
        version: cask.version.clone(),
        installed_at: now_timestamp(),
        artifact_path: install_result.clone(),
        auto_updates: cask.auto_updates,
        artifacts: vec![], // Will be populated by install functions
    };

    installed_casks.add(token, installed);
    installed_casks.save(state_path)?;

    info!("Installed {} to {}", token, install_result.display());
    Ok(install_result)
}

/// Install an artifact based on type
async fn install_artifact(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::install_artifact(cask, artifact_path, artifact_type, options).await
    }

    #[cfg(target_os = "linux")]
    {
        crate::linux::install_artifact(cask, artifact_path, artifact_type, options).await
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        Err(Error::UnsupportedPlatform(
            std::env::consts::OS.to_string(),
        ))
    }
}

/// Uninstall a cask
pub async fn uninstall_cask(
    token: &str,
    state_path: &Path,
    zap: bool,
) -> Result<()> {
    let mut installed_casks = InstalledCasks::load(state_path)?;

    let installed = installed_casks.get(token).ok_or_else(|| {
        Error::UninstallFailed(format!("{} is not installed", token))
    })?;

    let artifact_path = installed.artifact_path.clone();

    // Remove the installed artifact
    if artifact_path.exists() {
        if artifact_path.is_dir() {
            info!("Removing {}", artifact_path.display());
            std::fs::remove_dir_all(&artifact_path)
                .map_err(|e| Error::UninstallFailed(format!("Failed to remove {}: {}", artifact_path.display(), e)))?;
        } else if artifact_path.is_file() {
            info!("Removing {}", artifact_path.display());
            std::fs::remove_file(&artifact_path)
                .map_err(|e| Error::UninstallFailed(format!("Failed to remove {}: {}", artifact_path.display(), e)))?;
        }
    } else {
        warn!("Artifact path {} does not exist", artifact_path.display());
    }

    // Remove from state
    installed_casks.remove(token);
    installed_casks.save(state_path)?;

    if zap {
        info!("Zap requested - additional cleanup would happen here");
        // TODO: Implement zap (remove preferences, caches, etc.)
    }

    info!("Uninstalled {}", token);
    Ok(())
}
