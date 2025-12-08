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
    #[serde(default)]
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfig {
    /// Base URL for stout-index repository
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Require valid Ed25519 signatures on index updates
    /// Default: true in release builds, false in debug
    #[serde(default = "default_require_signature")]
    pub require_signature: bool,
    /// Allow unsigned indexes (for development/testing)
    /// Default: false in release builds, true in debug
    #[serde(default = "default_allow_unsigned")]
    pub allow_unsigned: bool,
    /// Maximum age of signature in seconds before rejecting
    /// Default: 7 days (604800 seconds)
    #[serde(default = "default_max_signature_age")]
    pub max_signature_age: u64,
    /// Additional trusted public keys (hex-encoded Ed25519 public keys)
    /// The default stout-index key is always trusted
    #[serde(default)]
    pub additional_trusted_keys: Vec<String>,
}

// Defaults
fn default_base_url() -> String {
    "https://raw.githubusercontent.com/neul-labs/stout-index/main".to_string()
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

fn default_require_signature() -> bool {
    cfg!(not(debug_assertions))
}

fn default_allow_unsigned() -> bool {
    cfg!(debug_assertions)
}

fn default_max_signature_age() -> u64 {
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

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            require_signature: default_require_signature(),
            allow_unsigned: default_allow_unsigned(),
            max_signature_age: default_max_signature_age(),
            additional_trusted_keys: vec![],
        }
    }
}

impl SecurityConfig {
    /// Convert to stout-index SecurityPolicy
    pub fn to_security_policy(&self) -> stout_index::SecurityPolicy {
        stout_index::SecurityPolicy {
            require_signature: self.require_signature,
            max_signature_age: self.max_signature_age,
            additional_keys: self.additional_trusted_keys.clone(),
            allow_unsigned: self.allow_unsigned,
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
            security: SecurityConfig::default(),
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
