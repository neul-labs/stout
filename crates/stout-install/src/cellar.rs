//! Cellar scanning and Homebrew receipt parsing
//!
//! Provides utilities for discovering packages installed in the Homebrew Cellar
//! and parsing their INSTALL_RECEIPT.json files.

use crate::error::Result;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Parsed data from a Homebrew INSTALL_RECEIPT.json
#[derive(Debug, Clone)]
pub struct BrewReceipt {
    pub installed_on_request: bool,
    pub install_time: Option<u64>,
    pub runtime_dependencies: Vec<BrewRuntimeDep>,
    pub source_tap: Option<String>,
    pub poured_from_bottle: Option<bool>,
}

/// A runtime dependency from a Homebrew receipt
#[derive(Debug, Clone)]
pub struct BrewRuntimeDep {
    pub full_name: String,
    pub version: String,
}

/// A package discovered in the Cellar
#[derive(Debug, Clone)]
pub struct CellarPackage {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub receipt: Option<BrewReceipt>,
}

// Internal serde structures for permissive parsing of INSTALL_RECEIPT.json
#[derive(Deserialize)]
struct RawReceipt {
    #[serde(default)]
    installed_on_request: Option<bool>,
    #[serde(default)]
    install_time: Option<serde_json::Value>,
    #[serde(default)]
    runtime_dependencies: Option<serde_json::Value>,
    #[serde(default)]
    source: Option<RawSource>,
    #[serde(default)]
    poured_from_bottle: Option<bool>,
}

#[derive(Deserialize)]
struct RawSource {
    #[serde(default)]
    tap: Option<String>,
}

#[derive(Deserialize)]
struct RawRuntimeDep {
    #[serde(default)]
    full_name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

/// Parse a Homebrew INSTALL_RECEIPT.json file.
///
/// Uses permissive parsing with serde defaults to handle variation
/// across Homebrew versions.
pub fn parse_brew_receipt(path: &Path) -> Result<BrewReceipt> {
    let json = std::fs::read_to_string(path)?;
    let raw: RawReceipt = serde_json::from_str(&json)?;

    let install_time = raw.install_time.and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_u64(),
        _ => None,
    });

    let runtime_dependencies = match raw.runtime_dependencies {
        Some(serde_json::Value::Array(arr)) => arr
            .into_iter()
            .filter_map(|v| {
                let dep: RawRuntimeDep = serde_json::from_value(v).ok()?;
                Some(BrewRuntimeDep {
                    full_name: dep.full_name?,
                    version: dep.version.unwrap_or_default(),
                })
            })
            .collect(),
        _ => Vec::new(),
    };

    Ok(BrewReceipt {
        installed_on_request: raw.installed_on_request.unwrap_or(true),
        install_time,
        runtime_dependencies,
        source_tap: raw.source.and_then(|s| s.tap),
        poured_from_bottle: raw.poured_from_bottle,
    })
}

/// Scan the Cellar directory and return all discovered packages.
///
/// For each package, selects the linked version (if any) or the latest version.
pub fn scan_cellar(cellar: &Path) -> Result<Vec<CellarPackage>> {
    if !cellar.exists() {
        return Ok(Vec::new());
    }

    let mut packages = Vec::new();

    let entries = std::fs::read_dir(cellar)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Skip hidden directories and reject unsafe names
        if name.starts_with('.') || !is_safe_package_name(&name) {
            continue;
        }

        match scan_package_versions(&path, &name) {
            Ok(Some(pkg)) => packages.push(pkg),
            Ok(None) => {
                debug!("No versions found for {}", name);
            }
            Err(e) => {
                warn!("Failed to scan {}: {}", name, e);
            }
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

/// Scan a single package in the Cellar by name.
pub fn scan_cellar_package(cellar: &Path, name: &str) -> Result<Option<CellarPackage>> {
    if !is_safe_package_name(name) {
        return Ok(None);
    }
    let pkg_dir = cellar.join(name);
    if !pkg_dir.is_dir() {
        return Ok(None);
    }
    scan_package_versions(&pkg_dir, name)
}

/// Reject package names that could escape the Cellar directory.
fn is_safe_package_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
        && name != ".."
}

/// Scan version subdirectories for a single package, picking the best version.
fn scan_package_versions(pkg_dir: &Path, name: &str) -> Result<Option<CellarPackage>> {
    let mut versions: Vec<String> = Vec::new();

    let entries = std::fs::read_dir(pkg_dir)?;
    for entry in entries {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        if let Ok(v) = entry.file_name().into_string() {
            if !v.starts_with('.') {
                versions.push(v);
            }
        }
    }

    if versions.is_empty() {
        return Ok(None);
    }

    // Sort versions — pick linked version or latest (last after sort)
    versions.sort();
    let version = versions.last().unwrap().clone();

    let version_path = pkg_dir.join(&version);
    let receipt_path = version_path.join("INSTALL_RECEIPT.json");

    let receipt = if receipt_path.exists() {
        match parse_brew_receipt(&receipt_path) {
            Ok(r) => Some(r),
            Err(e) => {
                warn!("Failed to parse receipt for {}/{}: {}", name, version, e);
                None
            }
        }
    } else {
        None
    };

    Ok(Some(CellarPackage {
        name: name.to_string(),
        version,
        path: version_path,
        receipt,
    }))
}

/// Count packages in the Cellar without fully parsing them.
pub fn count_cellar_packages(cellar: &Path) -> usize {
    if !cellar.exists() {
        return 0;
    }

    std::fs::read_dir(cellar)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path().is_dir()
                        && e.file_name()
                            .to_str()
                            .map(|n| !n.starts_with('.'))
                            .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}

/// Convert a Unix timestamp to an ISO 8601 string.
pub fn timestamp_to_iso(ts: u64) -> String {
    use jiff::Timestamp;
    Timestamp::from_second(ts as i64)
        .unwrap_or(Timestamp::UNIX_EPOCH)
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}
