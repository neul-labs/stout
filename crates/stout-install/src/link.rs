//! Symlink management

use crate::error::{Error, Result};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Directories to link from Cellar to prefix
const LINK_DIRS: &[&str] = &[
    "bin", "sbin", "lib", "include", "share", "etc", "var", "opt",
];

/// Link a package from Cellar to prefix
///
/// Creates symlinks: `<prefix>/bin/wget` -> `../Cellar/wget/1.24.5/bin/wget`
pub fn link_package(
    install_path: impl AsRef<Path>,
    prefix: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    let install_path = install_path.as_ref();
    let prefix = prefix.as_ref();

    let mut linked = Vec::new();

    for dir in LINK_DIRS {
        let source_dir = install_path.join(dir);
        if !source_dir.exists() {
            continue;
        }

        let target_dir = prefix.join(dir);
        std::fs::create_dir_all(&target_dir)?;

        // Link all files in this directory
        for entry in walkdir(&source_dir)? {
            let entry = entry?;
            let relative = entry.strip_prefix(&source_dir)
                .expect("entry should be under source_dir");
            let target = target_dir.join(relative);

            // Create parent directories for nested files
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }

            // Skip if target already exists
            if target.exists() || target.symlink_metadata().is_ok() {
                // Check if it's a symlink to our file
                if let Ok(link_target) = std::fs::read_link(&target) {
                    if link_target == entry {
                        continue; // Already linked correctly
                    }
                }
                warn!(
                    "Skipping {}: already exists",
                    target.display()
                );
                continue;
            }

            // Create relative symlink
            let relative_source = pathdiff::diff_paths(&entry, target.parent()
                .expect("target should have a parent directory"))
                .unwrap_or_else(|| entry.to_path_buf());

            debug!("Linking {} -> {}", target.display(), relative_source.display());
            symlink(&relative_source, &target)?;
            linked.push(target);
        }
    }

    // Create opt link: <prefix>/opt/<name> -> ../Cellar/<name>/<version>
    let opt_dir = prefix.join("opt");
    std::fs::create_dir_all(&opt_dir)?;

    if let Some(name) = install_path.parent().and_then(|p| p.file_name()) {
        let opt_link = opt_dir.join(name);
        if !opt_link.exists() {
            let relative = pathdiff::diff_paths(install_path, &opt_dir)
                .unwrap_or_else(|| install_path.to_path_buf());
            symlink(&relative, &opt_link)?;
            linked.push(opt_link);
        }
    }

    Ok(linked)
}

/// Unlink a package
pub fn unlink_package(
    install_path: impl AsRef<Path>,
    prefix: impl AsRef<Path>,
) -> Result<Vec<PathBuf>> {
    let install_path = install_path.as_ref();
    let prefix = prefix.as_ref();

    let mut unlinked = Vec::new();

    for dir in LINK_DIRS {
        let source_dir = install_path.join(dir);
        if !source_dir.exists() {
            continue;
        }

        let target_dir = prefix.join(dir);
        if !target_dir.exists() {
            continue;
        }

        for entry in walkdir(&source_dir)? {
            let entry = entry?;
            let relative = entry.strip_prefix(&source_dir)
                .expect("entry should be under source_dir");
            let target = target_dir.join(relative);

            if let Ok(link_target) = std::fs::read_link(&target) {
                // Check if it points to our file
                let resolved = target.parent()
                    .expect("target should have a parent directory")
                    .join(&link_target);
                if resolved.canonicalize().ok() == entry.canonicalize().ok() {
                    debug!("Unlinking {}", target.display());
                    std::fs::remove_file(&target)?;
                    unlinked.push(target);
                }
            }
        }
    }

    // Remove opt link
    let opt_dir = prefix.join("opt");
    if let Some(name) = install_path.parent().and_then(|p| p.file_name()) {
        let opt_link = opt_dir.join(name);
        if opt_link.symlink_metadata().is_ok() {
            std::fs::remove_file(&opt_link)?;
            unlinked.push(opt_link);
        }
    }

    Ok(unlinked)
}

/// Simple directory walker that yields files only
fn walkdir(dir: &Path) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    fn walk_recursive(dir: &Path, results: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                walk_recursive(&path, results)?;
            } else {
                results.push(path);
            }
        }
        Ok(())
    }

    let mut results = Vec::new();
    walk_recursive(dir, &mut results)?;
    Ok(results.into_iter().map(Ok))
}

// Add pathdiff as a dependency or inline a simple version
mod pathdiff {
    use std::path::{Path, PathBuf};

    pub fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
        let path = path.canonicalize().ok()?;
        let base = base.canonicalize().ok()?;

        let mut path_iter = path.components().peekable();
        let mut base_iter = base.components().peekable();

        // Skip common prefix
        while path_iter.peek() == base_iter.peek() {
            path_iter.next();
            if base_iter.next().is_none() {
                break;
            }
        }

        // Add ".." for remaining base components
        let mut result = PathBuf::new();
        for _ in base_iter {
            result.push("..");
        }

        // Add remaining path components
        for component in path_iter {
            result.push(component);
        }

        Some(result)
    }
}
