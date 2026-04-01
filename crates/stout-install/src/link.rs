//! Symlink management

use crate::error::Result;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Directories to link from Cellar to prefix
const LINK_DIRS: &[&str] = &[
    "bin", "sbin", "lib", "include", "share", "etc", "var", "opt",
];

use crate::extract::create_dir_all_force;

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
        create_dir_all_force(&target_dir)?;

        // Link all files in this directory
        for entry in walkdir(&source_dir)? {
            let entry = entry?;
            let relative = entry
                .strip_prefix(&source_dir)
                .expect("entry should be under source_dir");
            let target = target_dir.join(relative);

            // Create parent directories for nested files
            if let Some(parent) = target.parent() {
                create_dir_all_force(parent)?;
            }

            // Check if target already exists
            let target_exists = target.exists() || target.symlink_metadata().is_ok();

            if target_exists {
                // Check if it's a symlink pointing to our file
                if let Ok(link_target) = std::fs::read_link(&target) {
                    // Resolve the link target relative to its parent
                    let resolved = target
                        .parent()
                        .expect("target should have a parent directory")
                        .join(&link_target);

                    // Compare canonicalized paths if possible
                    let matches = if let (Ok(resolved_canon), Ok(entry_canon)) =
                        (resolved.canonicalize(), entry.canonicalize())
                    {
                        resolved_canon == entry_canon
                    } else {
                        // Fall back to string comparison if canonicalization fails
                        link_target == entry || resolved == entry
                    };

                    if matches {
                        continue; // Already linked correctly
                    }
                    // Symlink points elsewhere - warn and skip
                    warn!(
                        "Skipping {}: symlink points to {} instead of {}",
                        target.display(),
                        link_target.display(),
                        entry.display()
                    );
                    continue;
                }

                // Not a symlink - check if it's a regular file with matching content
                if target.is_file() && entry.is_file() {
                    if files_match(&target, &entry) {
                        debug!(
                            "Skipping {}: regular file with matching content",
                            target.display()
                        );
                        continue;
                    }
                    warn!(
                        "Skipping {}: regular file exists with different content",
                        target.display()
                    );
                    continue;
                }

                // Directory or other type - can't compare, just warn
                warn!(
                    "Skipping {}: already exists (not a symlink)",
                    target.display()
                );
                continue;
            }

            // Create relative symlink
            let relative_source = pathdiff::diff_paths(
                &entry,
                target
                    .parent()
                    .expect("target should have a parent directory"),
            )
            .unwrap_or_else(|| entry.to_path_buf());

            debug!(
                "Linking {} -> {}",
                target.display(),
                relative_source.display()
            );
            symlink(&relative_source, &target).map_err(|e| {
                crate::error::Error::LinkFailed(format!(
                    "{} -> {}: {}",
                    target.display(),
                    relative_source.display(),
                    e
                ))
            })?;
            linked.push(target);
        }
    }

    // Create opt link: <prefix>/opt/<name> -> ../Cellar/<name>/<version>
    let opt_dir = prefix.join("opt");
    create_dir_all_force(&opt_dir)?;

    if let Some(name) = install_path.parent().and_then(|p| p.file_name()) {
        let opt_link = opt_dir.join(name);
        let relative = pathdiff::diff_paths(install_path, &opt_dir)
            .unwrap_or_else(|| install_path.to_path_buf());

        // Check if opt_link exists (including broken symlinks via symlink_metadata)
        if opt_link.symlink_metadata().is_ok() {
            // Verify it points to the correct target
            if let Ok(link_target) = std::fs::read_link(&opt_link) {
                if link_target == relative {
                    // Already correctly linked
                    linked.push(opt_link);
                } else {
                    // Remove stale link and recreate
                    debug!("Removing stale opt link: {}", opt_link.display());
                    std::fs::remove_file(&opt_link)?;
                    symlink(&relative, &opt_link)?;
                    linked.push(opt_link);
                }
            } else {
                // Not a symlink (regular file), remove and create symlink
                debug!("Replacing opt link: {}", opt_link.display());
                std::fs::remove_file(&opt_link)?;
                symlink(&relative, &opt_link)?;
                linked.push(opt_link);
            }
        } else {
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
            let relative = entry
                .strip_prefix(&source_dir)
                .expect("entry should be under source_dir");
            let target = target_dir.join(relative);

            if let Ok(link_target) = std::fs::read_link(&target) {
                // Check if it points to our file
                let resolved = target
                    .parent()
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

/// Simple directory walker that yields files only.
///
/// Uses `symlink_metadata` to avoid following symlinks into directories,
/// which could cause infinite loops on circular symlinks.
fn walkdir(dir: &Path) -> Result<impl Iterator<Item = Result<PathBuf>>> {
    fn walk_recursive(dir: &Path, results: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ft = path.symlink_metadata()?.file_type();
            if ft.is_dir() {
                walk_recursive(&path, results)?;
            } else if !ft.is_symlink() || !path.is_dir() {
                // Include regular files and symlinks to files, but not
                // symlinks to directories (which could create cycles)
                results.push(path);
            }
        }
        Ok(())
    }

    let mut results = Vec::new();
    walk_recursive(dir, &mut results)?;
    Ok(results.into_iter().map(Ok))
}

/// Compare two files to see if they have identical content
///
/// First compares sizes, then does a byte-by-byte comparison if sizes match.
/// Returns false if either file cannot be read.
fn files_match(a: &Path, b: &Path) -> bool {
    // Quick size check first
    let meta_a = match std::fs::metadata(a) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let meta_b = match std::fs::metadata(b) {
        Ok(m) => m,
        Err(_) => return false,
    };

    if meta_a.len() != meta_b.len() {
        return false;
    }

    // Byte-by-byte comparison
    let Ok(mut file_a) = std::fs::File::open(a) else {
        return false;
    };
    let Ok(mut file_b) = std::fs::File::open(b) else {
        return false;
    };

    use std::io::Read;
    let mut buf_a = [0u8; 8192];
    let mut buf_b = [0u8; 8192];

    loop {
        let read_a = match file_a.read(&mut buf_a) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => return false,
        };
        let read_b = match file_b.read(&mut buf_b) {
            Ok(n) if n == read_a => n,
            _ => return false,
        };
        if buf_a[..read_a] != buf_b[..read_b] {
            return false;
        }
    }

    true
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
