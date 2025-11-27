//! User configuration

use crate::error::Result;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// User configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub index: IndexConfig,
    #[serde(default)]
    pub install: InstallConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub analytics: AnalyticsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Base URL for brewx-index repository
    #[serde(default = "default_base_url")]
    pub base_url: String,
    /// Automatically update index
    #[serde(default = "default_true")]
    pub auto_update: bool,
    /// Update interval in seconds
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    /// Homebrew Cellar path
    #[serde(default = "default_cellar")]
    pub cellar: String,
    /// Homebrew prefix path
    #[serde(default = "default_prefix")]
    pub prefix: String,
    /// Number of parallel downloads
    #[serde(default = "default_parallel")]
    pub parallel_downloads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum cache size
    #[serde(default = "default_max_size")]
    pub max_size: String,
    /// Formula cache TTL in seconds
    #[serde(default = "default_formula_ttl")]
    pub formula_ttl: u64,
    /// Download cache TTL in seconds
    #[serde(default = "default_download_ttl")]
    pub download_ttl: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Enable anonymous usage analytics (opt-in)
    #[serde(default)]
    pub enabled: bool,
}

// Defaults
fn default_base_url() -> String {
    "https://raw.githubusercontent.com/anthropics/brewx-index/main".to_string()
}

fn default_true() -> bool {
    true
}

fn default_update_interval() -> u64 {
    1800 // 30 minutes
}

fn default_cellar() -> String {
    "/opt/homebrew/Cellar".to_string()
}

fn default_prefix() -> String {
    "/opt/homebrew".to_string()
}

fn default_parallel() -> u32 {
    4
}

fn default_max_size() -> String {
    "2GB".to_string()
}

fn default_formula_ttl() -> u64 {
    86400 // 1 day
}

fn default_download_ttl() -> u64 {
    604800 // 7 days
}

impl Default for IndexConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            auto_update: default_true(),
            update_interval: default_update_interval(),
        }
    }
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            cellar: default_cellar(),
            prefix: default_prefix(),
            parallel_downloads: default_parallel(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: default_max_size(),
            formula_ttl: default_formula_ttl(),
            download_ttl: default_download_ttl(),
        }
    }
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Opt-in by default
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            index: IndexConfig::default(),
            install: InstallConfig::default(),
            cache: CacheConfig::default(),
            analytics: AnalyticsConfig::default(),
        }
    }
}

impl Config {
    /// Load config from file, or return defaults if not found
    pub fn load(paths: &Paths) -> Result<Self> {
        let config_path = paths.config_file();

        if config_path.exists() {
            let contents = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&contents)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to file
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let config_path = paths.config_file();

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, contents)?;
        Ok(())
    }
}
