//! Cask scanning - detect installed Homebrew casks from the Caskroom filesystem
//!
//! Scans the Caskroom directory directly instead of shelling out to `brew`,
//! which may not be installed if stout is the primary package manager.

use crate::error::Result;
use std::path::{Path, PathBuf};

/// Information about an installed Homebrew cask
#[derive(Debug, Clone)]
pub struct InstalledBrewCask {
    pub token: String,
    pub version: Option<String>,
    pub path: PathBuf,
}

/// Detect the Caskroom path from a Homebrew prefix.
pub fn caskroom_path(prefix: &Path) -> PathBuf {
    prefix.join("Caskroom")
}

/// Scan for installed casks by reading the Caskroom directory.
///
/// Each immediate subdirectory (that is not a symlink) is a cask.
/// Version is read from the non-hidden subdirectory within each cask dir.
pub fn scan_caskroom(prefix: &Path) -> Result<Vec<InstalledBrewCask>> {
    let caskroom = caskroom_path(prefix);
    if !caskroom.exists() {
        return Ok(Vec::new());
    }

    let mut casks = Vec::new();

    let entries = std::fs::read_dir(&caskroom)?;
    for entry in entries {
        let entry = entry?;

        // Skip symlinks (renamed cask aliases)
        if entry.file_type()?.is_symlink() {
            continue;
        }

        if !entry.path().is_dir() {
            continue;
        }

        let token = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip hidden directories
        if token.starts_with('.') {
            continue;
        }

        // Read version from subdirectory name
        let version = read_cask_version(&entry.path());

        casks.push(InstalledBrewCask {
            token,
            version,
            path: entry.path(),
        });
    }

    casks.sort_by(|a, b| a.token.cmp(&b.token));
    Ok(casks)
}

/// Read the installed version of a cask from its subdirectories.
///
/// Picks the latest non-hidden, non-`.upgrading` subdirectory name as the version.
fn read_cask_version(cask_dir: &Path) -> Option<String> {
    let entries = std::fs::read_dir(cask_dir).ok()?;

    let mut versions: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|name| !name.starts_with('.') && !name.ends_with(".upgrading"))
        .collect();

    versions.sort();
    versions.pop()
}

/// Count installed casks (fast, doesn't require full parsing)
pub fn count_caskroom_casks(prefix: &Path) -> usize {
    let caskroom = caskroom_path(prefix);
    if !caskroom.exists() {
        return 0;
    }

    std::fs::read_dir(&caskroom)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_dir()
                        && !e.file_type().map(|t| t.is_symlink()).unwrap_or(false)
                        && e.file_name()
                            .to_str()
                            .map(|n| !n.starts_with('.'))
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}
