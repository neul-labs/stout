//! Bottle extraction

use crate::error::{Error, Result};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::{debug, info};

/// Extract a bottle tarball to the Cellar
///
/// Bottles are tarballs with structure: `<name>/<version>/...`
/// We extract to: `<cellar>/<name>/<version>/...`
pub fn extract_bottle(
    bottle_path: impl AsRef<Path>,
    cellar: impl AsRef<Path>,
) -> Result<PathBuf> {
    let bottle_path = bottle_path.as_ref();
    let cellar = cellar.as_ref();

    debug!("Extracting {} to {}", bottle_path.display(), cellar.display());

    let file = File::open(bottle_path)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Create cellar if it doesn't exist
    std::fs::create_dir_all(cellar)?;

    // Extract all entries
    let mut install_path: Option<PathBuf> = None;

    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?;

        // Get the top-level directory (package name)
        if install_path.is_none() {
            if let Some(component) = path.components().next() {
                let pkg_name = component.as_os_str().to_string_lossy();
                // The path inside the tarball is like `wget/1.24.5/...`
                // We want to extract to `<cellar>/wget/1.24.5/...`
                if let Some(second) = path.components().nth(1) {
                    let version = second.as_os_str().to_string_lossy();
                    install_path = Some(cellar.join(&*pkg_name).join(&*version));
                }
            }
        }

        // Compute full destination path
        let dest = cellar.join(&path);

        // Create parent directories
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Extract the entry
        entry.unpack(&dest)?;
    }

    let install_path = install_path.ok_or_else(|| {
        Error::InvalidBottle("Could not determine install path from bottle".to_string())
    })?;

    info!("Extracted to {}", install_path.display());
    Ok(install_path)
}

/// Remove an installed package from the Cellar
pub fn remove_package(cellar: impl AsRef<Path>, name: &str, version: &str) -> Result<()> {
    let package_path = cellar.as_ref().join(name).join(version);

    if !package_path.exists() {
        return Err(Error::PackageNotFound(format!("{}/{}", name, version)));
    }

    debug!("Removing {}", package_path.display());
    std::fs::remove_dir_all(&package_path)?;

    // Remove parent directory if empty
    let parent = cellar.as_ref().join(name);
    if parent.read_dir()?.next().is_none() {
        std::fs::remove_dir(&parent)?;
    }

    info!("Removed {}-{}", name, version);
    Ok(())
}
