//! Bottle extraction

use crate::error::{Error, Result};
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::{debug, info, warn};

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

/// Homebrew placeholder strings that need to be replaced
const HOMEBREW_PLACEHOLDERS: &[(&str, &str)] = &[
    ("@@HOMEBREW_PREFIX@@", "prefix"),
    ("@@HOMEBREW_CELLAR@@", "cellar"),
    ("@@HOMEBREW_LIBRARY@@", "library"),
    ("@@HOMEBREW_REPOSITORY@@", "repository"),
];

/// Check if a file is an ELF binary
fn is_elf_binary(path: &Path) -> bool {
    if let Ok(mut file) = File::open(path) {
        let mut magic = [0u8; 4];
        if file.read_exact(&mut magic).is_ok() {
            // ELF magic number: 0x7f 'E' 'L' 'F'
            return magic == [0x7f, b'E', b'L', b'F'];
        }
    }
    false
}

/// Relocate ELF binary using patchelf
fn relocate_elf_binary(path: &Path, prefix: &str) -> Result<bool> {
    // Check if patchelf is available
    let patchelf = std::process::Command::new("patchelf")
        .arg("--version")
        .output();

    if patchelf.is_err() {
        // patchelf not available, skip with warning
        warn!(
            "patchelf not found - ELF binaries may not work correctly. \
             Install patchelf for proper binary relocation."
        );
        return Ok(false);
    }

    // Read current interpreter
    let output = std::process::Command::new("patchelf")
        .arg("--print-interpreter")
        .arg(path)
        .output();

    let interp = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => return Ok(false), // Not a dynamically linked executable
    };

    // Check if interpreter contains a placeholder
    if !interp.contains("@@HOMEBREW") && !interp.contains("linuxbrew") {
        return Ok(false);
    }

    // Compute new interpreter path
    let new_interp = if interp.contains("@@HOMEBREW_PREFIX@@") {
        interp.replace("@@HOMEBREW_PREFIX@@", prefix)
    } else if interp.contains("/home/linuxbrew/.linuxbrew") {
        interp.replace("/home/linuxbrew/.linuxbrew", prefix)
    } else {
        return Ok(false);
    };

    // Check if the new interpreter exists, if not use system ld
    let final_interp = if Path::new(&new_interp).exists() {
        new_interp
    } else {
        // Fall back to system dynamic linker
        let system_ld = find_system_ld();
        if let Some(ld) = system_ld {
            debug!("Using system linker {} instead of {}", ld, new_interp);
            ld
        } else {
            warn!("Cannot find suitable dynamic linker for {}", path.display());
            return Ok(false);
        }
    };

    // Set the new interpreter
    let result = std::process::Command::new("patchelf")
        .arg("--set-interpreter")
        .arg(&final_interp)
        .arg(path)
        .output();

    match result {
        Ok(o) if o.status.success() => {
            debug!("Patched ELF interpreter: {} -> {}", path.display(), final_interp);
            Ok(true)
        }
        Ok(o) => {
            warn!(
                "patchelf failed for {}: {}",
                path.display(),
                String::from_utf8_lossy(&o.stderr)
            );
            Ok(false)
        }
        Err(e) => {
            warn!("patchelf error for {}: {}", path.display(), e);
            Ok(false)
        }
    }
}

/// Find the system dynamic linker
fn find_system_ld() -> Option<String> {
    let candidates = [
        "/lib64/ld-linux-x86-64.so.2",
        "/lib/ld-linux-x86-64.so.2",
        "/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
        "/lib/ld-linux-aarch64.so.1",
        "/lib/aarch64-linux-gnu/ld-linux-aarch64.so.1",
    ];

    for candidate in candidates {
        if Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Relocate Homebrew placeholders in the extracted bottle
///
/// Replaces @@HOMEBREW_PREFIX@@ and similar placeholders with actual paths
pub fn relocate_bottle(
    install_path: impl AsRef<Path>,
    prefix: impl AsRef<Path>,
) -> Result<usize> {
    let install_path = install_path.as_ref();
    let prefix = prefix.as_ref();
    let cellar = prefix.join("Cellar");

    let prefix_str = prefix.to_string_lossy();
    let cellar_str = cellar.to_string_lossy();

    let mut relocated_count = 0;

    // Walk all files in the install path
    for entry in walkdir(install_path)? {
        let path = entry;

        // Skip directories and symlinks
        let metadata = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        if !metadata.is_file() {
            continue;
        }

        // Check if this is an ELF binary
        if is_elf_binary(&path) {
            // Use patchelf for ELF binaries
            if relocate_elf_binary(&path, &prefix_str)? {
                relocated_count += 1;
            }
        } else {
            // Use text replacement for non-ELF files
            if relocate_file(&path, &prefix_str, &cellar_str)? {
                relocated_count += 1;
            }
        }
    }

    if relocated_count > 0 {
        debug!("Relocated {} files", relocated_count);
    }

    Ok(relocated_count)
}

/// Recursively walk a directory and return all file paths
fn walkdir(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walkdir_recursive(dir.as_ref(), &mut files)?;
    Ok(files)
}

fn walkdir_recursive(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walkdir_recursive(&path, files)?;
            } else {
                files.push(path);
            }
        }
    }
    Ok(())
}

/// Relocate a single file, replacing Homebrew placeholders
fn relocate_file(path: &Path, prefix: &str, cellar: &str) -> Result<bool> {
    // Get original permissions
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            warn!("Could not get metadata for relocation: {}: {}", path.display(), e);
            return Ok(false);
        }
    };
    let original_permissions = metadata.permissions();

    // Make file writable if needed
    let was_readonly = original_permissions.mode() & 0o200 == 0;
    if was_readonly {
        let mut writable = original_permissions.clone();
        writable.set_mode(original_permissions.mode() | 0o200);
        fs::set_permissions(path, writable)?;
    }

    // Read the file
    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Could not open file for relocation: {}: {}", path.display(), e);
            if was_readonly {
                let _ = fs::set_permissions(path, original_permissions);
            }
            return Ok(false);
        }
    };

    let mut contents = Vec::new();
    if let Err(e) = file.read_to_end(&mut contents) {
        warn!("Could not read file for relocation: {}: {}", path.display(), e);
        if was_readonly {
            let _ = fs::set_permissions(path, original_permissions);
        }
        return Ok(false);
    }
    drop(file);

    // Check if file contains any placeholders
    let contains_placeholder = contents.windows(2).any(|w| w == b"@@");
    if !contains_placeholder {
        if was_readonly {
            fs::set_permissions(path, original_permissions)?;
        }
        return Ok(false);
    }

    // Perform replacements
    let mut modified = false;
    let mut new_contents = contents.clone();

    // Replace @@HOMEBREW_PREFIX@@
    new_contents = replace_bytes(&new_contents, b"@@HOMEBREW_PREFIX@@", prefix.as_bytes());
    if new_contents != contents {
        modified = true;
    }

    // Replace @@HOMEBREW_CELLAR@@
    let after_cellar = replace_bytes(&new_contents, b"@@HOMEBREW_CELLAR@@", cellar.as_bytes());
    if after_cellar != new_contents {
        modified = true;
        new_contents = after_cellar;
    }

    // Replace @@HOMEBREW_LIBRARY@@
    let library = format!("{}/Library", prefix);
    let after_library = replace_bytes(&new_contents, b"@@HOMEBREW_LIBRARY@@", library.as_bytes());
    if after_library != new_contents {
        modified = true;
        new_contents = after_library;
    }

    // Replace @@HOMEBREW_REPOSITORY@@
    let after_repo = replace_bytes(&new_contents, b"@@HOMEBREW_REPOSITORY@@", prefix.as_bytes());
    if after_repo != new_contents {
        modified = true;
        new_contents = after_repo;
    }

    if modified {
        // Write the modified file
        let mut file = File::create(path)?;
        file.write_all(&new_contents)?;
        debug!("Relocated: {}", path.display());
    }

    // Restore original permissions
    fs::set_permissions(path, original_permissions)?;

    Ok(modified)
}

/// Replace all occurrences of a byte pattern in a byte vector
///
/// Note: This does NOT pad with nulls, so the file size may change.
/// This works for both text files and binaries where the placeholder
/// is part of a longer path (e.g., @@HOMEBREW_PREFIX@@/lib/ld.so).
fn replace_bytes(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return haystack.to_vec();
    }

    let mut result = Vec::with_capacity(haystack.len());
    let mut i = 0;

    while i < haystack.len() {
        if haystack[i..].starts_with(needle) {
            result.extend_from_slice(replacement);
            i += needle.len();
        } else {
            result.push(haystack[i]);
            i += 1;
        }
    }

    result
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
