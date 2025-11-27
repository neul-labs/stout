//! Lockfile support for reproducible environments
//!
//! The lockfile (`brewx.lock`) captures the exact versions of all packages
//! installed, allowing for reproducible environments across machines.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Lockfile format version
const LOCKFILE_VERSION: u32 = 1;

/// A lockfile for reproducible package installations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    /// Lockfile format version
    pub version: u32,
    /// When the lockfile was created
    pub created_at: String,
    /// Platform this lockfile was created on
    pub platform: String,
    /// Locked packages
    pub packages: BTreeMap<String, LockedPackage>,
}

/// A locked package entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package version
    pub version: String,
    /// Package revision
    pub revision: u32,
    /// SHA256 of the bottle (if bottle install)
    pub bottle_sha256: Option<String>,
    /// Bottle URL (if bottle install)
    pub bottle_url: Option<String>,
    /// Source SHA256 (if built from source)
    pub source_sha256: Option<String>,
    /// Source URL (if built from source)
    pub source_url: Option<String>,
    /// Whether built from source
    #[serde(default)]
    pub built_from_source: bool,
    /// Dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl Lockfile {
    /// Create a new empty lockfile
    pub fn new() -> Self {
        let platform = format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );

        let created_at = chrono_lite_timestamp();

        Self {
            version: LOCKFILE_VERSION,
            created_at,
            platform,
            packages: BTreeMap::new(),
        }
    }

    /// Load a lockfile from disk
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let lockfile: Lockfile = toml::from_str(&contents)?;
        Ok(lockfile)
    }

    /// Save the lockfile to disk
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Add or update a package
    pub fn add_package(&mut self, name: &str, package: LockedPackage) {
        self.packages.insert(name.to_string(), package);
    }

    /// Remove a package
    pub fn remove_package(&mut self, name: &str) {
        self.packages.remove(name);
    }

    /// Get a package
    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.get(name)
    }

    /// Check if a package is locked
    pub fn is_locked(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Check if the lockfile matches the current platform
    pub fn matches_platform(&self) -> bool {
        let current = format!(
            "{}-{}",
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        self.platform == current
    }

    /// Get all package names
    pub fn package_names(&self) -> impl Iterator<Item = &str> {
        self.packages.keys().map(|s| s.as_str())
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

impl LockedPackage {
    /// Create a new locked package from a bottle installation
    pub fn from_bottle(
        version: &str,
        revision: u32,
        bottle_url: &str,
        bottle_sha256: &str,
        dependencies: Vec<String>,
    ) -> Self {
        Self {
            version: version.to_string(),
            revision,
            bottle_sha256: Some(bottle_sha256.to_string()),
            bottle_url: Some(bottle_url.to_string()),
            source_sha256: None,
            source_url: None,
            built_from_source: false,
            dependencies,
        }
    }

    /// Create a new locked package from a source build
    pub fn from_source(
        version: &str,
        revision: u32,
        source_url: &str,
        source_sha256: &str,
        dependencies: Vec<String>,
    ) -> Self {
        Self {
            version: version.to_string(),
            revision,
            bottle_sha256: None,
            bottle_url: None,
            source_sha256: Some(source_sha256.to_string()),
            source_url: Some(source_url.to_string()),
            built_from_source: true,
            dependencies,
        }
    }
}

/// Simple timestamp without external dependencies
fn chrono_lite_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lockfile_creation() {
        let lockfile = Lockfile::new();
        assert_eq!(lockfile.version, LOCKFILE_VERSION);
        assert!(lockfile.packages.is_empty());
    }

    #[test]
    fn test_add_package() {
        let mut lockfile = Lockfile::new();
        let pkg = LockedPackage::from_bottle(
            "1.0.0",
            0,
            "https://example.com/pkg.tar.gz",
            "abc123",
            vec!["dep1".to_string()],
        );
        lockfile.add_package("test-pkg", pkg);
        assert!(lockfile.is_locked("test-pkg"));
    }
}
