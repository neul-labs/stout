//! Cask installation logic

use crate::detect_artifact_type_from_cask;
use crate::download::{download_cask_artifact, ArtifactType};
use crate::error::{Error, Result};
use crate::state::{now_timestamp, InstalledCask, InstalledCasks};
use std::path::{Path, PathBuf};
use stout_index::Cask;
use tracing::{debug, warn};

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
    let url = cask
        .download_url()
        .ok_or_else(|| Error::InstallFailed(format!("No download URL for cask {}", token)))?;

    // Detect artifact type from cask metadata (preferred) or URL fallback
    let artifact_type = detect_artifact_type_from_cask(cask, url);

    // Get expected checksum
    let sha256 = if options.no_verify {
        None
    } else {
        cask.sha256.as_str()
    };

    debug!("Downloading {}...", token);

    // Warn if verification is disabled
    if options.no_verify {
        warn!("Checksum verification is disabled - this is a security risk");
    }

    if options.dry_run {
        debug!("[dry-run] Would download {} from {}", token, url);
        debug!("[dry-run] Would install to /Applications");
        return Ok(PathBuf::from("/Applications"));
    }

    // Download artifact
    let artifact_path =
        download_cask_artifact(url, cache_dir, token, sha256, artifact_type).await?;

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

    debug!("Installed {} to {}", token, install_result.display());
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
        Err(Error::UnsupportedPlatform(std::env::consts::OS.to_string()))
    }
}

/// Uninstall a cask
pub async fn uninstall_cask(token: &str, state_path: &Path, zap: bool) -> Result<()> {
    let mut installed_casks = InstalledCasks::load(state_path)?;

    let installed = installed_casks
        .get(token)
        .ok_or_else(|| Error::UninstallFailed(format!("{} is not installed", token)))?;

    let artifact_path = installed.artifact_path.clone();

    // Remove the installed artifact
    if artifact_path.exists() {
        if artifact_path.is_dir() {
            debug!("Removing {}", artifact_path.display());
            std::fs::remove_dir_all(&artifact_path).map_err(|e| {
                Error::UninstallFailed(format!(
                    "Failed to remove {}: {}",
                    artifact_path.display(),
                    e
                ))
            })?;
        } else if artifact_path.is_file() {
            debug!("Removing {}", artifact_path.display());
            std::fs::remove_file(&artifact_path).map_err(|e| {
                Error::UninstallFailed(format!(
                    "Failed to remove {}: {}",
                    artifact_path.display(),
                    e
                ))
            })?;
        }
    } else {
        warn!("Artifact path {} does not exist", artifact_path.display());
    }

    // Remove from state
    installed_casks.remove(token);
    installed_casks.save(state_path)?;

    if zap {
        debug!("Zap requested - note: full zap (preferences, caches, support files) not yet implemented");
        // TODO: Implement zap - would require tracking additional file locations
        // Typical locations: ~/Library/Preferences/, ~/Library/Caches/, ~/Application Support/
    }

    debug!("Uninstalled {}", token);
    Ok(())
}

/// Install only the artifact (no state management) - for parallel installation
/// Returns the installed path
pub async fn install_artifact_only(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    if options.dry_run {
        debug!("[dry-run] Would install {} to /Applications", cask.token);
        return Ok(PathBuf::from("/Applications"));
    }

    // Install based on platform and artifact type
    let install_result = install_artifact(cask, artifact_path, artifact_type, options).await?;

    debug!("Installed {} to {}", cask.token, install_result.display());
    Ok(install_result)
}

/// Synchronous version of install_artifact_only for use with spawn_blocking
/// This is more efficient than spawn_blocking + block_on
#[cfg(target_os = "macos")]
pub fn install_artifact_sync(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    if options.dry_run {
        debug!("[dry-run] Would install {} to /Applications", cask.token);
        return Ok(PathBuf::from("/Applications"));
    }

    let install_result =
        crate::macos::install_artifact_sync(cask, artifact_path, artifact_type, options)?;

    debug!("Installed {} to {}", cask.token, install_result.display());
    Ok(install_result)
}

#[cfg(target_os = "linux")]
pub fn install_artifact_sync(
    cask: &Cask,
    artifact_path: &Path,
    artifact_type: ArtifactType,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    if options.dry_run {
        debug!("[dry-run] Would install {} to /Applications", cask.token);
        return Ok(PathBuf::from("/Applications"));
    }

    let install_result =
        crate::linux::install_artifact_sync(cask, artifact_path, artifact_type, options)?;

    debug!("Installed {} to {}", cask.token, install_result.display());
    Ok(install_result)
}
