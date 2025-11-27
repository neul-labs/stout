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
}

fn default_installed_by() -> String {
    "brewx".to_string()
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
                installed_by: "brewx".to_string(),
                requested,
                pinned,
                dependencies,
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

/// Simple timestamp without pulling in chrono
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple ISO 8601 format
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Approximate year calculation (doesn't account for leap years perfectly)
    let years = 1970 + (days_since_epoch / 365);
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        years, month, day, hours, minutes, seconds
    )
}
