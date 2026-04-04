//! macOS-specific cask installation

use crate::download::ArtifactType;
use crate::error::{Error, Result};
use crate::install::CaskInstallOptions;
use std::path::{Path, PathBuf};
use std::process::Command;
use stout_index::Cask;
use tracing::{debug, warn};

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
    // Delegate to sync version - all operations are synchronous anyway
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
        ArtifactType::Dmg => install_from_dmg_sync(cask, artifact_path, options),
        ArtifactType::Pkg => install_from_pkg_sync(cask, artifact_path, options),
        ArtifactType::Zip => install_from_zip_sync(cask, artifact_path, options),
        ArtifactType::TarGz | ArtifactType::TarBz2 => {
            install_from_archive_sync(cask, artifact_path, options)
        }
        ArtifactType::AppImage => Err(Error::UnsupportedPlatform(
            "AppImage is not supported on macOS".to_string(),
        )),
    }
}

/// Install from DMG (synchronous)
fn install_from_dmg_sync(
    cask: &Cask,
    dmg_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    debug!("Mounting DMG for {}...", cask.token);
    let mount_point = mount_dmg(dmg_path)?;
    debug!("DMG mounted, installing artifacts for {}...", cask.token);

    // Install based on cask artifacts metadata
    let result = install_cask_artifacts(cask, &mount_point, options);

    // Always try to unmount, even on error
    if let Err(e) = unmount_dmg(&mount_point) {
        warn!("Failed to unmount DMG: {}", e);
    }

    result
}

/// Install from PKG (synchronous)
fn install_from_pkg_sync(
    cask: &Cask,
    pkg_path: &Path,
    _options: &CaskInstallOptions,
) -> Result<PathBuf> {
    debug!("Installing package {}...", pkg_path.display());

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

    // PKG installation requires sudo - use installer command.
    // Inherit stdin/stderr so the user sees sudo's password prompt.
    // Discard stdout to suppress installer's verbose progress output.
    let mut child = Command::new("sudo")
        .args(["installer", "-pkg"])
        .arg(pkg_path)
        .args(["-target", "/"])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| Error::CommandFailed {
            cmd: "installer".to_string(),
            message: e.to_string(),
        })?;

    let status = child.wait().map_err(|e| Error::CommandFailed {
        cmd: "installer".to_string(),
        message: e.to_string(),
    })?;

    if !status.success() {
        return Err(Error::InstallFailed(format!(
            "Package installation failed with exit code {}",
            status.code().unwrap_or(-1)
        )));
    }

    // Remove quarantine from installed apps based on cask artifacts
    // PKG installers typically put apps in /Applications
    let appdir = PathBuf::from("/Applications");
    let mut last_installed: Option<PathBuf> = None;

    for artifact in &cask.artifacts {
        if artifact.artifact_type == "app" {
            if let Some(source) = &artifact.source {
                let dest = appdir.join(source.trim_start_matches('/'));
                if dest.exists() {
                    debug!("Removing quarantine from {}", dest.display());
                    if let Err(e) = remove_quarantine(&dest) {
                        warn!("Failed to remove quarantine from {}: {}", dest.display(), e);
                    }
                    last_installed = Some(dest);
                }
            }
        }
    }

    // If no artifacts found, try common app name based on cask token
    if last_installed.is_none() {
        // Try common naming patterns
        let app_names = vec![
            format!(
                "{}.app",
                cask.token.split('-').next().unwrap_or(&cask.token)
            ),
            // Capitalize first letter
            format!(
                "{}{}.app",
                cask.token.chars().next().unwrap_or('a').to_uppercase(),
                &cask.token[1..]
            ),
        ];

        for app_name in app_names {
            let dest = appdir.join(&app_name);
            if dest.exists() {
                debug!("Removing quarantine from {}", dest.display());
                if let Err(e) = remove_quarantine(&dest) {
                    warn!("Failed to remove quarantine from {}: {}", dest.display(), e);
                }
                last_installed = Some(dest);
                break;
            }
        }
    }

    // Return the installed path or a placeholder
    Ok(last_installed.unwrap_or_else(|| PathBuf::from("/Applications")))
}

/// Install from ZIP (synchronous)
fn install_from_zip_sync(
    cask: &Cask,
    zip_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;
    let temp_dir = std::env::temp_dir().join(format!("stout-{}", token));

    // Clean up temp dir from any previous failed attempt
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir)?;

    // Use RAII guard for automatic cleanup
    let _guard = TempDirGuard(temp_dir.clone());

    // Extract ZIP
    debug!("Extracting ZIP for {}...", token);
    let output = Command::new("unzip")
        .args(["-q", "-o"])
        .arg(zip_path)
        .args(["-d"])
        .arg(&temp_dir)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "unzip".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!("Unzip failed: {}", stderr)));
    }

    debug!("ZIP extracted, installing artifacts for {}...", token);

    // Install based on cask artifacts metadata
    install_cask_artifacts(cask, &temp_dir, options)
}

/// Install artifacts based on cask metadata
fn install_cask_artifacts(
    cask: &Cask,
    extracted_dir: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let appdir = options
        .appdir
        .clone()
        .unwrap_or_else(|| PathBuf::from("/Applications"));
    let bindir = PathBuf::from("/opt/homebrew/bin");

    let mut last_installed: Option<PathBuf> = None;

    // Process each artifact from cask metadata
    for artifact in &cask.artifacts {
        match artifact.artifact_type.as_str() {
            "app" => {
                // Install .app bundle
                if let Some(source) = &artifact.source {
                    let app_path = extracted_dir.join(source);
                    if app_path.exists() {
                        let app_name = app_path.file_name().unwrap().to_string_lossy().to_string();
                        let dest = appdir.join(&app_name);

                        // Remove existing if force
                        if dest.exists() && options.force {
                            debug!("Removing existing {}", dest.display());
                            std::fs::remove_dir_all(&dest)?;
                        }

                        if dest.exists() && !options.force {
                            debug!("{} already exists, skipping", dest.display());
                        } else {
                            debug!("Installing {} to {}", app_name, appdir.display());
                            copy_dir_all(&app_path, &dest)?;
                            remove_quarantine(&dest)?;
                        }
                        last_installed = Some(dest);
                    }
                }
            }
            "binary" => {
                // Install binary to /opt/homebrew/bin
                if let Some(source) = &artifact.source {
                    // Source paths from the Homebrew cask index often carry a full
                    // Caskroom-style prefix:
                    //   $HOMEBREW_PREFIX/Caskroom/<token>/<version>/<archive-relative-path>
                    // Strip everything up through "<token>/<version>/" to get the path
                    // that actually exists inside the extracted archive.
                    let relative = strip_cask_source_prefix(source, &cask.token, &cask.version);
                    let binary_path = extracted_dir.join(relative);
                    if binary_path.exists() {
                        let binary_name = binary_path
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .to_string();
                        let dest = bindir.join(&binary_name);

                        // Remove existing if force
                        if dest.exists() && options.force {
                            std::fs::remove_file(&dest)?;
                        }

                        if dest.exists() {
                            debug!("{} already exists, skipping", dest.display());
                        } else {
                            debug!("Installing binary {} to {}", binary_name, bindir.display());
                            std::fs::copy(&binary_path, &dest)?;

                            // Make executable
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                std::fs::set_permissions(
                                    &dest,
                                    std::fs::Permissions::from_mode(0o755),
                                )?;
                            }
                        }
                        last_installed = Some(dest);
                    }
                }
            }
            "pkg" => {
                // PKG artifacts are handled separately via install_from_pkg
                // This shouldn't normally be reached for ZIP containers
            }
            "zap" | "uninstall" | "postflight" | "preflight" => {
                // Skip non-install artifacts
                continue;
            }
            _ => {
                // Unknown artifact type - try to find .app as fallback
                debug!("Unknown artifact type: {}", artifact.artifact_type);
            }
        }
    }

    // If no artifacts were processed, fall back to finding .app bundles
    if last_installed.is_none() {
        if let Ok(app_bundle) = find_app_in_dir(extracted_dir) {
            let app_name = app_bundle
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let dest = appdir.join(&app_name);

            if dest.exists() && options.force {
                debug!("Removing existing {}", dest.display());
                std::fs::remove_dir_all(&dest)?;
            }

            if !dest.exists() {
                debug!("Installing {} to {}", app_name, appdir.display());
                copy_dir_all(&app_bundle, &dest)?;
                remove_quarantine(&dest)?;
                last_installed = Some(dest);
            }
        }
    }

    last_installed.ok_or_else(|| {
        Error::ArtifactNotFound(format!("No installable artifacts found in {}", cask.token))
    })
}

/// Install from tar.gz or tar.bz2 (synchronous)
fn install_from_archive_sync(
    cask: &Cask,
    archive_path: &Path,
    options: &CaskInstallOptions,
) -> Result<PathBuf> {
    let token = &cask.token;
    let temp_dir = std::env::temp_dir().join(format!("stout-{}", token));

    // Clean up temp dir from any previous failed attempt
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir)?;

    // Use RAII guard for automatic cleanup
    let _guard = TempDirGuard(temp_dir.clone());

    // Extract archive
    debug!("Extracting archive for {}...", token);
    let output = Command::new("tar")
        .args(["-xf"])
        .arg(archive_path)
        .args(["-C"])
        .arg(&temp_dir)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "tar".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!("Extract failed: {}", stderr)));
    }

    // Install based on cask artifacts metadata
    install_cask_artifacts(cask, &temp_dir, options)
}

/// Mount a DMG file
fn mount_dmg(dmg_path: &Path) -> Result<PathBuf> {
    let mount_point = std::env::temp_dir().join(format!(
        "stout-mount-{}",
        dmg_path.file_stem().unwrap().to_string_lossy()
    ));

    // Clean up mount point from any previous failed attempt
    if mount_point.exists() {
        // Try to detach first (in case it's still mounted)
        let _ = Command::new("hdiutil")
            .args(["detach", "-quiet", "-force"])
            .arg(&mount_point)
            .stdin(std::process::Stdio::null())
            .output();
        // Remove leftover directory
        let _ = std::fs::remove_dir_all(&mount_point);
    }

    // Create fresh mount point
    std::fs::create_dir_all(&mount_point)?;

    debug!(
        "Mounting {} at {}",
        dmg_path.display(),
        mount_point.display()
    );

    // Remove quarantine from the DMG before mounting to prevent macOS from
    // blocking the attach with a Gatekeeper verification dialog.
    let _ = Command::new("xattr")
        .args(["-rd", "com.apple.quarantine"])
        .arg(dmg_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    // Phase 1: Try mounting quietly (stdout suppressed). Most DMGs mount
    // without interaction and the disk info output is noisy.
    let mut child = Command::new("hdiutil")
        .args([
            "attach",
            "-nobrowse",
            "-readonly",
            "-noverify",
            "-mountpoint",
        ])
        .arg(&mount_point)
        .arg(dmg_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| Error::CommandFailed {
            cmd: "hdiutil attach".to_string(),
            message: e.to_string(),
        })?;

    const SLA_DETECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
    const ATTACH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);
    let start = std::time::Instant::now();

    // Wait for quiet mount to complete, or detect SLA prompt
    let status = loop {
        match child.try_wait().map_err(|e| Error::CommandFailed {
            cmd: "hdiutil attach".to_string(),
            message: e.to_string(),
        })? {
            Some(s) => break Some(s),
            None => {
                if start.elapsed() >= ATTACH_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::MountFailed(format!(
                        "hdiutil attach timed out after {}s \
                         (license agreement or security dialog may be blocking)",
                        ATTACH_TIMEOUT.as_secs()
                    )));
                }
                if start.elapsed() >= SLA_DETECT_TIMEOUT {
                    // Likely an SLA prompt — kill and retry interactively
                    break None;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    };

    let status = match status {
        Some(s) => s,
        None => {
            // Phase 2: SLA-licensed DMG detected — retry with interactive
            // stdout so the user can see and respond to the license prompt.
            let _ = child.kill();
            let _ = child.wait();

            eprintln!(
                "  {} Interactive license agreement detected — \
                 please respond to the prompt below.",
                console::style("⚠").yellow()
            );

            let mut child = Command::new("hdiutil")
                .args([
                    "attach",
                    "-nobrowse",
                    "-readonly",
                    "-noverify",
                    "-mountpoint",
                ])
                .arg(&mount_point)
                .arg(dmg_path)
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| Error::CommandFailed {
                    cmd: "hdiutil attach".to_string(),
                    message: e.to_string(),
                })?;

            let sla_start = std::time::Instant::now();
            loop {
                match child.try_wait().map_err(|e| Error::CommandFailed {
                    cmd: "hdiutil attach".to_string(),
                    message: e.to_string(),
                })? {
                    Some(s) => break s,
                    None => {
                        if sla_start.elapsed() >= ATTACH_TIMEOUT {
                            let _ = child.kill();
                            let _ = child.wait();
                            return Err(Error::MountFailed(format!(
                                "hdiutil attach timed out after {}s \
                                 (license agreement or security dialog may be blocking)",
                                ATTACH_TIMEOUT.as_secs()
                            )));
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        }
    };

    if !status.success() {
        return Err(Error::MountFailed(
            "hdiutil attach failed (non-zero exit)".to_string(),
        ));
    }

    Ok(mount_point)
}

/// Unmount a DMG
fn unmount_dmg(mount_point: &Path) -> Result<()> {
    debug!("Unmounting {}", mount_point.display());

    let output = Command::new("hdiutil")
        .args(["detach", "-quiet"])
        .arg(mount_point)
        .stdin(std::process::Stdio::null())
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

        // Skip macOS resource fork files (._*)
        if let Some(name) = path.file_name() {
            if name.to_string_lossy().starts_with("._") {
                continue;
            }
        }

        if path.extension().map(|e| e == "app").unwrap_or(false) {
            return Ok(path);
        }

        // Check one level deeper
        if path.is_dir() {
            for subentry in std::fs::read_dir(&path)? {
                let subentry = subentry?;
                let subpath = subentry.path();

                // Skip macOS resource fork files (._*)
                if let Some(name) = subpath.file_name() {
                    if name.to_string_lossy().starts_with("._") {
                        continue;
                    }
                }

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

/// Copy a directory recursively, preserving macOS extended attributes
/// (code signatures, resource forks, ACLs, etc.)
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    // Use ditto on macOS to preserve extended attributes, resource forks, and ACLs.
    // std::fs::copy does NOT preserve these, which breaks code signatures on .app bundles.
    let output = Command::new("ditto")
        .arg(src)
        .arg(dst)
        .stdin(std::process::Stdio::null())
        .output()
        .map_err(|e| Error::CommandFailed {
            cmd: "ditto".to_string(),
            message: e.to_string(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::InstallFailed(format!(
            "Failed to copy {}: {}",
            src.display(),
            stderr
        )));
    }

    Ok(())
}

/// Remove quarantine attribute from a path (recursive)
fn remove_quarantine(path: &Path) -> Result<()> {
    debug!("Removing quarantine from {}", path.display());

    let _ = Command::new("xattr")
        .args(["-rd", "com.apple.quarantine"])
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output();

    Ok(())
}

/// Strip Homebrew's Caskroom prefix from a binary artifact source path.
///
/// Homebrew cask binary stanzas record the full Caskroom path, e.g.
///   `$HOMEBREW_PREFIX/Caskroom/android-platform-tools/37.0.0/platform-tools/adb`
/// stout extracts archives into a temp dir, so only the archive-relative part
/// (`platform-tools/adb`) is meaningful.
fn strip_cask_source_prefix<'a>(source: &'a str, token: &str, version: &str) -> &'a str {
    let marker = format!("{}/{}/", token, version);
    if let Some(pos) = source.find(marker.as_str()) {
        return &source[pos + marker.len()..];
    }
    source
}
