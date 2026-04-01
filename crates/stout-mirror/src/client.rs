//! Mirror client functionality

use crate::error::{Error, Result};
use crate::manifest::{MirrorManifest, PackageInfo};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Configuration for the mirror client
#[derive(Debug, Clone)]
pub struct MirrorClientConfig {
    /// Mirror URL (http:// or file://)
    pub url: String,

    /// How to handle packages not in mirror
    pub fallback: Fallback,

    /// Whether to verify checksums
    pub verify_checksums: bool,

    /// Local cache directory
    pub cache_dir: PathBuf,
}

impl Default for MirrorClientConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("stout")
            .join("mirror");

        Self {
            url: String::new(),
            fallback: Fallback::Error,
            verify_checksums: false,
            cache_dir,
        }
    }
}

/// How to handle packages not found in mirror
#[derive(Debug, Clone, Copy, Default)]
pub enum Fallback {
    /// Hard error if package not in mirror
    #[default]
    Error,
    /// Warn and try upstream
    Warn,
    /// Silently try upstream
    Silent,
}

impl std::str::FromStr for Fallback {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" => Ok(Self::Warn),
            "silent" => Ok(Self::Silent),
            _ => Err(format!("Unknown fallback mode: {}", s)),
        }
    }
}

/// Mirror client for installing from a local or remote mirror
pub struct MirrorClient {
    config: MirrorClientConfig,
    manifest: Option<MirrorManifest>,
    is_file_mirror: bool,
}

impl MirrorClient {
    /// Create a new mirror client
    pub fn new(config: MirrorClientConfig) -> Self {
        let is_file_mirror = config.url.starts_with("file://");
        Self {
            config,
            manifest: None,
            is_file_mirror,
        }
    }

    /// Connect to the mirror and fetch manifest
    pub async fn connect(&mut self) -> Result<()> {
        info!("Connecting to mirror: {}", self.config.url);

        let manifest = if self.is_file_mirror {
            self.load_file_manifest()?
        } else {
            self.fetch_http_manifest().await?
        };

        info!(
            "Connected: {} formulas, {} casks available",
            manifest.formulas.count, manifest.casks.count
        );

        self.manifest = Some(manifest);
        Ok(())
    }

    /// Load manifest from file:// URL
    fn load_file_manifest(&self) -> Result<MirrorManifest> {
        let path = self.file_path("manifest.json")?;
        MirrorManifest::load(&path)
    }

    /// Fetch manifest from http:// URL
    async fn fetch_http_manifest(&self) -> Result<MirrorManifest> {
        let url = format!("{}/manifest.json", self.config.url.trim_end_matches('/'));
        debug!("Fetching manifest from {}", url);

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        let manifest = response.json().await?;

        Ok(manifest)
    }

    /// Get file path for file:// URLs
    fn file_path(&self, relative: &str) -> Result<PathBuf> {
        let base = self
            .config
            .url
            .strip_prefix("file://")
            .ok_or_else(|| Error::Manifest("Invalid file:// URL".to_string()))?;

        Ok(PathBuf::from(base).join(relative))
    }

    /// Check if a formula is in the mirror
    pub fn has_formula(&self, name: &str) -> bool {
        self.manifest
            .as_ref()
            .map(|m| m.formulas.packages.contains_key(name))
            .unwrap_or(false)
    }

    /// Get formula info from manifest
    pub fn get_formula(&self, name: &str) -> Option<&PackageInfo> {
        self.manifest.as_ref()?.get_formula(name)
    }

    /// Download a bottle from the mirror
    pub async fn download_bottle(
        &self,
        name: &str,
        platform: &str,
        dest: &Path,
    ) -> Result<PathBuf> {
        let manifest = self
            .manifest
            .as_ref()
            .ok_or_else(|| Error::Manifest("Not connected to mirror".to_string()))?;

        let formula = manifest
            .get_formula(name)
            .ok_or_else(|| Error::PackageNotFound(name.to_string()))?;

        let bottle = formula
            .bottles
            .get(platform)
            .ok_or_else(|| Error::PlatformNotAvailable(platform.to_string(), name.to_string()))?;

        let bottle_path = if self.is_file_mirror {
            self.copy_file_bottle(&bottle.path, dest)?
        } else {
            self.download_http_bottle(&bottle.path, dest).await?
        };

        // Verify checksum if enabled
        if self.config.verify_checksums {
            let actual = sha256_file(&bottle_path)?;
            if actual != bottle.sha256 {
                return Err(Error::ChecksumMismatch(
                    name.to_string(),
                    bottle.sha256.clone(),
                    actual,
                ));
            }
            debug!("Checksum verified for {}", name);
        }

        Ok(bottle_path)
    }

    /// Copy bottle from file:// mirror
    fn copy_file_bottle(&self, relative_path: &str, dest: &Path) -> Result<PathBuf> {
        let src = self.file_path(relative_path)?;
        let dest_file = dest.join(
            src.file_name()
                .ok_or_else(|| Error::Manifest("Invalid bottle path".to_string()))?,
        );

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::copy(&src, &dest_file)?;
        Ok(dest_file)
    }

    /// Download bottle from http:// mirror
    async fn download_http_bottle(&self, relative_path: &str, dest: &Path) -> Result<PathBuf> {
        let url = format!(
            "{}/{}",
            self.config.url.trim_end_matches('/'),
            relative_path
        );

        debug!("Downloading bottle from {}", url);

        let client = reqwest::Client::new();
        let response = client.get(&url).send().await?;
        let bytes = response.bytes().await?;

        let filename = relative_path
            .rsplit('/')
            .next()
            .ok_or_else(|| Error::Manifest("Invalid bottle path".to_string()))?;

        let dest_file = dest.join(filename);

        if let Some(parent) = dest_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&dest_file, &bytes)?;
        Ok(dest_file)
    }

    /// Get available platforms for a formula
    pub fn get_platforms(&self, name: &str) -> Vec<String> {
        self.manifest
            .as_ref()
            .and_then(|m| m.get_formula(name))
            .map(|f| f.bottles.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// List all formulas in the mirror
    pub fn list_formulas(&self) -> Vec<&str> {
        self.manifest
            .as_ref()
            .map(|m| m.formulas.packages.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }

    /// Handle package not found based on fallback config
    pub fn handle_not_found(&self, name: &str) -> Result<()> {
        match self.config.fallback {
            Fallback::Error => Err(Error::PackageNotFound(name.to_string())),
            Fallback::Warn => {
                eprintln!("Warning: '{}' not in mirror, trying upstream...", name);
                Ok(())
            }
            Fallback::Silent => Ok(()),
        }
    }
}

/// Calculate SHA256 hash of a file
fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}
