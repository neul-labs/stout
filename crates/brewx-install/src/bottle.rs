//! Bottle creation from installed packages
//!
//! This module provides functionality to create Homebrew-compatible bottle
//! archives from packages that have been installed (either from source or
//! from existing bottles).

use crate::error::{Error, Result};
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::Builder;
use tracing::{debug, info};

/// Result of bottle creation
#[derive(Debug)]
pub struct BottleResult {
    /// Path to the created bottle
    pub path: PathBuf,
    /// Size in bytes
    pub size: u64,
    /// SHA256 hash of the bottle
    pub sha256: String,
    /// Number of files included
    pub file_count: usize,
}

/// Create a bottle from an installed package
///
/// # Arguments
///
/// * `install_path` - Path to the installed package (e.g., /opt/homebrew/Cellar/jq/1.7.1)
/// * `output_path` - Path where the bottle should be written
/// * `name` - Package name
/// * `version` - Package version
///
/// # Returns
///
/// A `BottleResult` containing information about the created bottle
pub fn create_bottle(
    install_path: &Path,
    output_path: &Path,
    name: &str,
    version: &str,
) -> Result<BottleResult> {
    info!(
        "Creating bottle for {} {} from {:?}",
        name, version, install_path
    );

    if !install_path.exists() {
        return Err(Error::Bottle(format!(
            "Install path does not exist: {:?}",
            install_path
        )));
    }

    // Create the tar.gz archive
    let output_file = File::create(output_path)
        .map_err(|e| Error::Bottle(format!("Failed to create output file: {}", e)))?;

    let encoder = GzEncoder::new(output_file, Compression::default());
    let mut builder = Builder::new(encoder);

    // The bottle should contain the package in the format:
    // <name>/<version>/<contents>
    let base_path = format!("{}/{}", name, version);
    let mut file_count = 0;

    // Recursively add all files
    file_count += add_directory_to_tar(&mut builder, install_path, &base_path)?;

    // Finish the archive
    let encoder = builder
        .into_inner()
        .map_err(|e| Error::Bottle(format!("Failed to finish archive: {}", e)))?;

    encoder
        .finish()
        .map_err(|e| Error::Bottle(format!("Failed to finish gzip: {}", e)))?;

    // Calculate SHA256
    let file_bytes = std::fs::read(output_path)
        .map_err(|e| Error::Bottle(format!("Failed to read bottle: {}", e)))?;

    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    let size = file_bytes.len() as u64;

    info!("Created bottle: {} bytes, {} files", size, file_count);

    Ok(BottleResult {
        path: output_path.to_path_buf(),
        size,
        sha256,
        file_count,
    })
}

/// Add a directory and its contents to a tar archive
fn add_directory_to_tar<W: Write>(
    builder: &mut Builder<W>,
    dir_path: &Path,
    archive_base: &str,
) -> Result<usize> {
    let mut count = 0;

    for entry in std::fs::read_dir(dir_path)
        .map_err(|e| Error::Bottle(format!("Failed to read directory: {}", e)))?
    {
        let entry =
            entry.map_err(|e| Error::Bottle(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        let file_name = entry.file_name();
        let archive_path = format!("{}/{}", archive_base, file_name.to_string_lossy());

        if path.is_dir() {
            // Recursively add directory
            count += add_directory_to_tar(builder, &path, &archive_path)?;
        } else if path.is_symlink() {
            // Handle symlinks
            let link_target = std::fs::read_link(&path)
                .map_err(|e| Error::Bottle(format!("Failed to read symlink: {}", e)))?;

            let mut header = tar::Header::new_gnu();
            header.set_entry_type(tar::EntryType::Symlink);
            header.set_size(0);

            // Get file metadata for permissions
            if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                header.set_mode(get_mode(&metadata));
                header.set_mtime(
                    metadata
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                );
            }

            builder
                .append_link(&mut header, &archive_path, &link_target)
                .map_err(|e| Error::Bottle(format!("Failed to add symlink to archive: {}", e)))?;

            debug!("Added symlink: {} -> {:?}", archive_path, link_target);
            count += 1;
        } else if path.is_file() {
            // Regular file
            let metadata = std::fs::metadata(&path)
                .map_err(|e| Error::Bottle(format!("Failed to get file metadata: {}", e)))?;

            let mut header = tar::Header::new_gnu();
            header.set_size(metadata.len());
            header.set_mode(get_mode(&metadata));
            header.set_mtime(
                metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );

            let mut file = File::open(&path)
                .map_err(|e| Error::Bottle(format!("Failed to open file: {}", e)))?;

            builder
                .append_data(&mut header, &archive_path, &mut file)
                .map_err(|e| Error::Bottle(format!("Failed to add file to archive: {}", e)))?;

            debug!("Added file: {}", archive_path);
            count += 1;
        }
    }

    Ok(count)
}

/// Get the file mode from metadata
fn get_mode(metadata: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        if metadata.is_dir() {
            0o755
        } else {
            0o644
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_bottle() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().join("test-pkg/1.0.0");
        let bin_dir = install_path.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();

        // Create a test binary
        fs::write(bin_dir.join("test-binary"), b"#!/bin/sh\necho hello").unwrap();

        let bottle_path = temp_dir.path().join("test-pkg-1.0.0.bottle.tar.gz");

        let result = create_bottle(&install_path, &bottle_path, "test-pkg", "1.0.0").unwrap();

        assert!(result.path.exists());
        assert!(result.size > 0);
        assert!(!result.sha256.is_empty());
        assert!(result.file_count > 0);
    }
}
