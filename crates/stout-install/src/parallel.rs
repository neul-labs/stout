//! Parallel installation support
//!
//! Provides concurrent extraction and linking of multiple packages
//! for faster installation times.

use crate::error::Result;
use crate::extract::extract_bottle;
use crate::link::link_package;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tracing::{debug, info};

/// Configuration for parallel installation
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Maximum number of concurrent extractions
    pub max_concurrent_extractions: usize,
    /// Maximum number of concurrent linking operations
    pub max_concurrent_links: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        // Default to number of CPUs for extractions (CPU-bound)
        // and 4 for links (mostly I/O bound, can conflict)
        let cpus = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        Self {
            max_concurrent_extractions: cpus,
            max_concurrent_links: 4,
        }
    }
}

/// Result of a single package installation
#[derive(Debug)]
pub struct PackageInstallResult {
    /// Package name
    pub name: String,
    /// Path where package was installed in Cellar
    pub install_path: PathBuf,
    /// Linked files
    pub linked_files: Vec<PathBuf>,
}

/// Parallel installer for multiple packages
pub struct ParallelInstaller {
    config: ParallelConfig,
    extract_semaphore: Arc<Semaphore>,
    link_semaphore: Arc<Semaphore>,
}

impl ParallelInstaller {
    /// Create a new parallel installer with default configuration
    pub fn new() -> Self {
        Self::with_config(ParallelConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(config: ParallelConfig) -> Self {
        let extract_semaphore = Arc::new(Semaphore::new(config.max_concurrent_extractions));
        let link_semaphore = Arc::new(Semaphore::new(config.max_concurrent_links));

        Self {
            config,
            extract_semaphore,
            link_semaphore,
        }
    }

    /// Extract multiple bottles in parallel
    ///
    /// Returns a vector of (name, install_path) pairs in the same order as input
    pub async fn extract_bottles(
        &self,
        bottles: Vec<BottleInfo>,
        cellar: &Path,
    ) -> Result<Vec<(String, PathBuf)>> {
        info!(
            "Extracting {} bottles with {} concurrent workers",
            bottles.len(),
            self.config.max_concurrent_extractions
        );

        let cellar = cellar.to_path_buf();
        let semaphore: Arc<Semaphore> = Arc::clone(&self.extract_semaphore);
        let mut join_set = JoinSet::new();

        // Store order mapping
        let order: Vec<String> = bottles.iter().map(|b| b.name.clone()).collect();

        for bottle in bottles {
            let cellar = cellar.clone();
            let semaphore = Arc::clone(&semaphore);

            join_set.spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await
                    .map_err(|e| crate::error::Error::Other(format!("Semaphore error: {}", e)))?;

                // Run blocking extraction in a spawn_blocking
                let name = bottle.name.clone();
                let bottle_path = bottle.bottle_path.clone();
                let cellar_clone = cellar.clone();

                let install_path = tokio::task::spawn_blocking(move || {
                    extract_bottle(&bottle_path, &cellar_clone)
                })
                .await
                .map_err(|e| crate::error::Error::Other(format!("Task join error: {}", e)))??;

                debug!("Extracted {} to {}", name, install_path.display());
                Ok::<_, crate::error::Error>((name, install_path))
            });
        }

        // Collect results
        let mut results: Vec<(String, PathBuf)> = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(item)) => results.push(item),
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(crate::error::Error::Other(format!(
                        "Task panic: {}",
                        e
                    )))
                }
            }
        }

        // Restore original order
        let mut ordered: Vec<(String, PathBuf)> = Vec::with_capacity(results.len());
        for name in &order {
            if let Some(pos) = results.iter().position(|(n, _)| n == name) {
                ordered.push(results.remove(pos));
            }
        }

        info!("Extracted {} bottles", ordered.len());
        Ok(ordered)
    }

    /// Link multiple packages in parallel
    ///
    /// Note: Linking can have conflicts if packages try to link the same file,
    /// so we use a smaller concurrency limit by default.
    pub async fn link_packages(
        &self,
        packages: Vec<LinkInfo>,
        prefix: &Path,
    ) -> Result<Vec<(String, Vec<PathBuf>)>> {
        info!(
            "Linking {} packages with {} concurrent workers",
            packages.len(),
            self.config.max_concurrent_links
        );

        let prefix = prefix.to_path_buf();
        let semaphore: Arc<Semaphore> = Arc::clone(&self.link_semaphore);
        let mut join_set = JoinSet::new();

        let order: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();

        for pkg in packages {
            let prefix = prefix.clone();
            let semaphore = Arc::clone(&semaphore);

            join_set.spawn(async move {
                let _permit = semaphore.acquire().await
                    .map_err(|e| crate::error::Error::Other(format!("Semaphore error: {}", e)))?;

                let name = pkg.name.clone();
                let install_path = pkg.install_path.clone();
                let prefix_clone = prefix.clone();

                let linked = tokio::task::spawn_blocking(move || {
                    link_package(&install_path, &prefix_clone)
                })
                .await
                .map_err(|e| crate::error::Error::Other(format!("Task join error: {}", e)))??;

                debug!("Linked {} ({} files)", name, linked.len());
                Ok::<_, crate::error::Error>((name, linked))
            });
        }

        let mut results: Vec<(String, Vec<PathBuf>)> = Vec::new();
        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(item)) => results.push(item),
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(crate::error::Error::Other(format!(
                        "Task panic: {}",
                        e
                    )))
                }
            }
        }

        // Restore original order
        let mut ordered: Vec<(String, Vec<PathBuf>)> = Vec::with_capacity(results.len());
        for name in &order {
            if let Some(pos) = results.iter().position(|(n, _)| n == name) {
                ordered.push(results.remove(pos));
            }
        }

        info!("Linked {} packages", ordered.len());
        Ok(ordered)
    }

    /// Install multiple packages in parallel (extract then link)
    ///
    /// Extracts all bottles in parallel first, then links in parallel.
    /// This two-phase approach avoids potential conflicts.
    pub async fn install_bottles(
        &self,
        bottles: Vec<BottleInfo>,
        cellar: &Path,
        prefix: &Path,
    ) -> Result<Vec<PackageInstallResult>> {
        // Phase 1: Extract all bottles in parallel
        let extracted = self.extract_bottles(bottles, cellar).await?;

        // Phase 2: Link all packages in parallel
        let link_infos: Vec<LinkInfo> = extracted
            .iter()
            .map(|(name, install_path)| LinkInfo {
                name: name.clone(),
                install_path: install_path.clone(),
            })
            .collect();

        let linked = self.link_packages(link_infos, prefix).await?;

        // Combine results
        let results: Vec<PackageInstallResult> = extracted
            .into_iter()
            .zip(linked.into_iter())
            .map(|((name, install_path), (_, linked_files))| PackageInstallResult {
                name,
                install_path,
                linked_files,
            })
            .collect();

        Ok(results)
    }
}

impl Default for ParallelInstaller {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a bottle to extract
#[derive(Debug, Clone)]
pub struct BottleInfo {
    /// Package name
    pub name: String,
    /// Path to the downloaded bottle tarball
    pub bottle_path: PathBuf,
}

/// Information about a package to link
#[derive(Debug, Clone)]
pub struct LinkInfo {
    /// Package name
    pub name: String,
    /// Path to the installed package in Cellar
    pub install_path: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_config_default() {
        let config = ParallelConfig::default();
        assert!(config.max_concurrent_extractions >= 1);
        assert!(config.max_concurrent_links >= 1);
    }
}
