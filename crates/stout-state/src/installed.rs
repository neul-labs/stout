//! Installed packages tracking

use crate::error::Result;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Information about an installed package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub version: String,
    #[serde(default)]
    pub revision: u32,
    pub installed_at: String,
    #[serde(default = "default_installed_by")]
    pub installed_by: String,
    #[serde(default)]
    pub requested: bool,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Full commit SHA for HEAD installations
    #[serde(default)]
    pub head_sha: Option<String>,
    /// Quick flag for HEAD detection
    #[serde(default)]
    pub is_head: bool,
}

fn default_installed_by() -> String {
    "stout".to_string()
}

impl InstalledPackage {
    /// Check if this is a HEAD installation
    pub fn is_head_install(&self) -> bool {
        self.is_head || self.version.starts_with("HEAD")
    }

    /// Get the short SHA for display (from version string)
    pub fn short_sha(&self) -> Option<&str> {
        if self.version.starts_with("HEAD-") {
            Some(&self.version[5..])
        } else {
            None
        }
    }
}

/// Collection of installed packages
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InstalledPackages {
    #[serde(default)]
    pub packages: HashMap<String, InstalledPackage>,
}

impl InstalledPackages {
    /// Load installed packages from file
    pub fn load(paths: &Paths) -> Result<Self> {
        let file_path = paths.installed_file();

        if file_path.exists() {
            let contents = std::fs::read_to_string(&file_path)?;
            let packages: InstalledPackages = toml::from_str(&contents)?;
            Ok(packages)
        } else {
            Ok(Self::default())
        }
    }

    /// Save installed packages to file
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let file_path = paths.installed_file();

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&file_path, contents)?;
        Ok(())
    }

    /// Add or update a package
    pub fn add(&mut self, name: &str, version: &str, revision: u32, requested: bool) {
        self.add_with_deps(name, version, revision, requested, Vec::new());
    }

    /// Add or update a package with dependencies
    pub fn add_with_deps(
        &mut self,
        name: &str,
        version: &str,
        revision: u32,
        requested: bool,
        dependencies: Vec<String>,
    ) {
        let now = chrono_lite_now();
        // Preserve pinned status if updating existing package
        let pinned = self.packages.get(name).map(|p| p.pinned).unwrap_or(false);
        self.packages.insert(
            name.to_string(),
            InstalledPackage {
                version: version.to_string(),
                revision,
                installed_at: now,
                installed_by: "stout".to_string(),
                requested,
                pinned,
                dependencies,
                head_sha: None,
                is_head: version.starts_with("HEAD"),
            },
        );
    }

    /// Add or update a package with full metadata (used by import/sync)
    pub fn add_imported(
        &mut self,
        name: &str,
        version: &str,
        revision: u32,
        requested: bool,
        installed_by: &str,
        installed_at: &str,
        dependencies: Vec<String>,
    ) {
        // Preserve pinned status if updating existing package
        let pinned = self.packages.get(name).map(|p| p.pinned).unwrap_or(false);
        self.packages.insert(
            name.to_string(),
            InstalledPackage {
                version: version.to_string(),
                revision,
                installed_at: installed_at.to_string(),
                installed_by: installed_by.to_string(),
                requested,
                pinned,
                dependencies,
                head_sha: None,
                is_head: version.starts_with("HEAD"),
            },
        );
    }

    /// Add a HEAD package with SHA tracking
    pub fn add_head(
        &mut self,
        name: &str,
        short_sha: &str,
        full_sha: &str,
        requested: bool,
        dependencies: Vec<String>,
    ) {
        let now = chrono_lite_now();
        // Preserve pinned status if updating existing package
        let pinned = self.packages.get(name).map(|p| p.pinned).unwrap_or(false);
        self.packages.insert(
            name.to_string(),
            InstalledPackage {
                version: format!("HEAD-{}", short_sha),
                revision: 0,
                installed_at: now,
                installed_by: "stout".to_string(),
                requested,
                pinned,
                dependencies,
                head_sha: Some(full_sha.to_string()),
                is_head: true,
            },
        );
    }

    /// Pin a package to prevent upgrades
    pub fn pin(&mut self, name: &str) -> bool {
        if let Some(pkg) = self.packages.get_mut(name) {
            pkg.pinned = true;
            true
        } else {
            false
        }
    }

    /// Unpin a package to allow upgrades
    pub fn unpin(&mut self, name: &str) -> bool {
        if let Some(pkg) = self.packages.get_mut(name) {
            pkg.pinned = false;
            true
        } else {
            false
        }
    }

    /// Check if a package is pinned
    pub fn is_pinned(&self, name: &str) -> bool {
        self.packages.get(name).map(|p| p.pinned).unwrap_or(false)
    }

    /// List pinned packages
    pub fn pinned(&self) -> impl Iterator<Item = (&String, &InstalledPackage)> {
        self.packages.iter().filter(|(_, p)| p.pinned)
    }

    /// Remove a package
    pub fn remove(&mut self, name: &str) -> Option<InstalledPackage> {
        self.packages.remove(name)
    }

    /// Get a package
    pub fn get(&self, name: &str) -> Option<&InstalledPackage> {
        self.packages.get(name)
    }

    /// Check if a package is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.packages.contains_key(name)
    }

    /// Check if a specific version is installed
    pub fn is_version_installed(&self, name: &str, version: &str) -> bool {
        self.packages
            .get(name)
            .map(|p| p.version == version)
            .unwrap_or(false)
    }

    /// Get all installed package names
    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.packages.keys()
    }

    /// Get count of installed packages
    pub fn count(&self) -> usize {
        self.packages.len()
    }

    /// List packages that were explicitly requested (not dependencies)
    pub fn requested(&self) -> impl Iterator<Item = (&String, &InstalledPackage)> {
        self.packages.iter().filter(|(_, p)| p.requested)
    }

    /// List packages that are dependencies
    pub fn dependencies(&self) -> impl Iterator<Item = (&String, &InstalledPackage)> {
        self.packages.iter().filter(|(_, p)| !p.requested)
    }

    /// Iterate over all installed packages
    pub fn iter(&self) -> impl Iterator<Item = (&String, &InstalledPackage)> {
        self.packages.iter()
    }
}

/// Current time as ISO 8601 UTC string.
fn chrono_lite_now() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}
