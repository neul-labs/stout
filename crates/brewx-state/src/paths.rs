//! Path management

use std::path::{Path, PathBuf};

/// Standard paths for brewx
#[derive(Debug, Clone)]
pub struct Paths {
    /// brewx config and cache directory (~/.brewx)
    pub brewx_dir: PathBuf,
    /// Homebrew prefix (/opt/homebrew or /usr/local)
    pub prefix: PathBuf,
    /// Cellar directory
    pub cellar: PathBuf,
}

impl Paths {
    /// Create paths with default locations
    pub fn default() -> Self {
        let brewx_dir = dirs::home_dir()
            .map(|h| h.join(".brewx"))
            .unwrap_or_else(|| PathBuf::from(".brewx"));

        let prefix = detect_homebrew_prefix();
        let cellar = prefix.join("Cellar");

        Self {
            brewx_dir,
            prefix,
            cellar,
        }
    }

    /// Create paths with custom locations
    pub fn new(brewx_dir: PathBuf, prefix: PathBuf) -> Self {
        let cellar = prefix.join("Cellar");
        Self {
            brewx_dir,
            prefix,
            cellar,
        }
    }

    /// Config file path
    pub fn config_file(&self) -> PathBuf {
        self.brewx_dir.join("config.toml")
    }

    /// Index database path
    pub fn index_db(&self) -> PathBuf {
        self.brewx_dir.join("index.db")
    }

    /// Manifest file path
    pub fn manifest(&self) -> PathBuf {
        self.brewx_dir.join("manifest.json")
    }

    /// Installed packages file
    pub fn installed_file(&self) -> PathBuf {
        self.brewx_dir.join("state").join("installed.toml")
    }

    /// Formula cache directory
    pub fn formula_cache(&self) -> PathBuf {
        self.brewx_dir.join("cache").join("formulas")
    }

    /// Download cache directory
    pub fn download_cache(&self) -> PathBuf {
        self.brewx_dir.join("cache").join("downloads")
    }

    /// Ensure all directories exist
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.brewx_dir)?;
        std::fs::create_dir_all(self.brewx_dir.join("state"))?;
        std::fs::create_dir_all(self.formula_cache())?;
        std::fs::create_dir_all(self.download_cache())?;
        Ok(())
    }

    /// Get the install path for a package
    pub fn package_path(&self, name: &str, version: &str) -> PathBuf {
        self.cellar.join(name).join(version)
    }

    /// Check if a package is installed
    pub fn is_installed(&self, name: &str, version: &str) -> bool {
        self.package_path(name, version).exists()
    }

    /// List installed versions of a package
    pub fn installed_versions(&self, name: &str) -> Vec<String> {
        let pkg_dir = self.cellar.join(name);
        if !pkg_dir.exists() {
            return Vec::new();
        }

        std::fs::read_dir(&pkg_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Detect the Homebrew prefix based on platform
fn detect_homebrew_prefix() -> PathBuf {
    // Check common locations
    let candidates = [
        "/opt/homebrew",       // macOS ARM
        "/usr/local",          // macOS Intel / Linux
        "/home/linuxbrew/.linuxbrew", // Linux
    ];

    for path in candidates {
        let p = Path::new(path);
        if p.join("Cellar").exists() {
            return p.to_path_buf();
        }
    }

    // Default to /opt/homebrew for new installs
    PathBuf::from("/opt/homebrew")
}

impl Default for Paths {
    fn default() -> Self {
        Self::default()
    }
}
