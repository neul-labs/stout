//! Path management

use std::path::{Path, PathBuf};

/// Standard paths for stout
#[derive(Debug, Clone)]
pub struct Paths {
    /// stout config and cache directory (~/.stout)
    pub stout_dir: PathBuf,
    /// Homebrew prefix (/opt/homebrew or /usr/local)
    pub prefix: PathBuf,
    /// Cellar directory
    pub cellar: PathBuf,
}

impl Paths {
    /// Create paths with custom locations
    pub fn new(stout_dir: PathBuf, prefix: PathBuf) -> Self {
        let cellar = prefix.join("Cellar");
        Self {
            stout_dir,
            prefix,
            cellar,
        }
    }

    /// Config file path
    pub fn config_file(&self) -> PathBuf {
        self.stout_dir.join("config.toml")
    }

    /// Index database path
    pub fn index_db(&self) -> PathBuf {
        self.stout_dir.join("index.db")
    }

    /// Manifest file path
    pub fn manifest(&self) -> PathBuf {
        self.stout_dir.join("manifest.json")
    }

    /// Installed packages file
    pub fn installed_file(&self) -> PathBuf {
        self.stout_dir.join("state").join("installed.toml")
    }

    /// Package history file
    pub fn history_file(&self) -> PathBuf {
        self.stout_dir.join("state").join("history.json")
    }

    /// Formula cache directory
    pub fn formula_cache(&self) -> PathBuf {
        self.stout_dir.join("cache").join("formulas")
    }

    /// Download cache directory
    pub fn download_cache(&self) -> PathBuf {
        self.stout_dir.join("cache").join("downloads")
    }

    /// Ensure all directories exist
    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.stout_dir)?;
        std::fs::create_dir_all(self.stout_dir.join("state"))?;
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
    // Check common locations for existing Homebrew installations
    let candidates = [
        "/opt/homebrew",              // macOS ARM
        "/usr/local",                 // macOS Intel / Linux
        "/home/linuxbrew/.linuxbrew", // Linux
    ];

    for path in candidates {
        let p = Path::new(path);
        if p.join("Cellar").exists() {
            return p.to_path_buf();
        }
    }

    // For new installs, use platform-appropriate location
    #[cfg(target_os = "macos")]
    {
        // macOS: use /opt/homebrew (ARM) or /usr/local (Intel)
        #[cfg(target_arch = "aarch64")]
        return PathBuf::from("/opt/homebrew");
        #[cfg(not(target_arch = "aarch64"))]
        return PathBuf::from("/usr/local");
    }

    #[cfg(target_os = "linux")]
    {
        // Linux: use ~/.local/stout for user-level installs (no sudo required)
        if let Some(home) = dirs::home_dir() {
            return home.join(".local").join("stout");
        }
        // Fallback to linuxbrew location
        PathBuf::from("/home/linuxbrew/.linuxbrew")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from("/opt/homebrew")
    }
}

impl Default for Paths {
    fn default() -> Self {
        let stout_dir = dirs::home_dir()
            .map(|h| h.join(".stout"))
            .unwrap_or_else(|| PathBuf::from(".stout"));

        let prefix = detect_homebrew_prefix();
        let cellar = prefix.join("Cellar");

        Self {
            stout_dir,
            prefix,
            cellar,
        }
    }
}
