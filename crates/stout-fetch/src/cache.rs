//! Download cache management

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};
use tracing::debug;

/// Cache for downloaded bottles
pub struct DownloadCache {
    cache_dir: PathBuf,
}

impl DownloadCache {
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get the path for a cached bottle
    pub fn bottle_path(&self, name: &str, version: &str, platform: &str) -> PathBuf {
        self.cache_dir
            .join("downloads")
            .join(format!("{}-{}-{}.tar.gz", name, version, platform))
    }

    /// Check if a bottle is cached
    pub fn has_bottle(&self, name: &str, version: &str, platform: &str) -> bool {
        self.bottle_path(name, version, platform).exists()
    }

    /// Get a cached bottle path if it exists
    pub fn get_bottle(&self, name: &str, version: &str, platform: &str) -> Option<PathBuf> {
        let path = self.bottle_path(name, version, platform);
        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Store a bottle in the cache
    pub fn store_bottle(
        &self,
        name: &str,
        version: &str,
        platform: &str,
        data: &[u8],
    ) -> Result<PathBuf> {
        let path = self.bottle_path(name, version, platform);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, data)?;
        debug!("Cached bottle at {}", path.display());

        Ok(path)
    }

    /// Remove a bottle from the cache
    pub fn remove_bottle(&self, name: &str, version: &str, platform: &str) -> Result<()> {
        let path = self.bottle_path(name, version, platform);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Get total cache size in bytes
    pub fn cache_size(&self) -> Result<u64> {
        let downloads_dir = self.cache_dir.join("downloads");
        if !downloads_dir.exists() {
            return Ok(0);
        }

        let mut total = 0u64;
        for entry in std::fs::read_dir(&downloads_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                total += entry.metadata()?.len();
            }
        }

        Ok(total)
    }

    /// Clean old cache entries
    pub fn clean(&self, max_age_secs: u64) -> Result<u64> {
        let downloads_dir = self.cache_dir.join("downloads");
        if !downloads_dir.exists() {
            return Ok(0);
        }

        let now = std::time::SystemTime::now();
        let mut freed = 0u64;

        for entry in std::fs::read_dir(&downloads_dir)? {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_file() {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() > max_age_secs {
                            freed += metadata.len();
                            std::fs::remove_file(entry.path())?;
                            debug!("Removed old cache entry: {}", entry.path().display());
                        }
                    }
                }
            }
        }

        Ok(freed)
    }
}
