//! HTTP client for downloading bottles

use crate::cache::DownloadCache;
use crate::error::{Error, Result};
use crate::progress::ProgressReporter;
use crate::verify::sha256_bytes;
use futures::future::join_all;
use reqwest::Client;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, info, warn};

/// Download client for bottles
#[derive(Clone)]
pub struct DownloadClient {
    client: Client,
    cache: Arc<DownloadCache>,
    /// Semaphore to limit concurrent downloads
    semaphore: Arc<Semaphore>,
}

impl DownloadClient {
    /// Create a new download client with specified concurrency limit
    pub fn new(cache: DownloadCache, max_concurrent: usize) -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("stout/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        Ok(Self {
            client,
            cache: Arc::new(cache),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        })
    }

    /// Create with default concurrency (4)
    pub fn with_cache(cache: DownloadCache) -> Result<Self> {
        Self::new(cache, 4)
    }

    /// Download a bottle, returning the path to the downloaded file
    pub async fn download_bottle(
        &self,
        name: &str,
        version: &str,
        platform: &str,
        url: &str,
        expected_sha256: &str,
        progress: Option<&ProgressReporter>,
    ) -> Result<PathBuf> {
        // Check cache first
        if let Some(path) = self.cache.get_bottle(name, version, platform) {
            // Verify cached file
            if crate::verify::verify_sha256(&path, expected_sha256).is_ok() {
                debug!("Using cached bottle for {}", name);
                return Ok(path);
            }
            // Cache corrupted, remove it
            self.cache.remove_bottle(name, version, platform)?;
        }

        // Download
        debug!("Downloading {} from {}", name, url);
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP {}: {}",
                response.status(),
                url
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let pb = progress.map(|p| p.new_download(name, total_size));

        // Stream to memory (bottles are typically small, 1-50MB)
        let bytes = response.bytes().await?;

        if let Some(ref pb) = pb {
            pb.set_position(bytes.len() as u64);
        }

        // Verify checksum
        let actual_sha256 = sha256_bytes(&bytes);
        if actual_sha256 != expected_sha256 {
            return Err(Error::ChecksumMismatch {
                path: url.to_string(),
                expected: expected_sha256.to_string(),
                actual: actual_sha256,
            });
        }

        // Cache the bottle
        let path = self.cache.store_bottle(name, version, platform, &bytes)?;

        if let Some(pb) = pb {
            pb.finish();
        }

        info!("Downloaded {}-{}", name, version);
        Ok(path)
    }

    /// Download a single bottle with semaphore (for use in parallel downloads)
    async fn download_bottle_with_semaphore(
        &self,
        spec: BottleSpec,
        progress: Arc<ProgressReporter>,
    ) -> Result<PathBuf> {
        // Acquire semaphore permit to limit concurrency
        let _permit = self.semaphore.acquire().await.map_err(|_| {
            Error::DownloadFailed("Semaphore closed".to_string())
        })?;

        self.download_bottle(
            &spec.name,
            &spec.version,
            &spec.platform,
            &spec.url,
            &spec.sha256,
            Some(&progress),
        )
        .await
    }

    /// Download multiple bottles in parallel
    pub async fn download_bottles_parallel(
        &self,
        bottles: Vec<BottleSpec>,
        progress: Arc<ProgressReporter>,
    ) -> Vec<Result<PathBuf>> {
        let futures: Vec<_> = bottles
            .into_iter()
            .map(|spec| {
                let client = self.clone();
                let progress = Arc::clone(&progress);
                async move { client.download_bottle_with_semaphore(spec, progress).await }
            })
            .collect();

        join_all(futures).await
    }

    /// Download multiple bottles in parallel, failing fast on first error
    pub async fn download_bottles(
        &self,
        bottles: Vec<BottleSpec>,
        progress: Arc<ProgressReporter>,
    ) -> Result<Vec<PathBuf>> {
        let results = self.download_bottles_parallel(bottles, progress).await;

        // Collect results, returning first error if any
        let mut paths = Vec::with_capacity(results.len());
        for result in results {
            paths.push(result?);
        }
        Ok(paths)
    }

    /// Get the cache
    pub fn cache(&self) -> &DownloadCache {
        &self.cache
    }
}

/// Specification for a bottle to download
#[derive(Debug, Clone)]
pub struct BottleSpec {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub url: String,
    pub sha256: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_client_creation() {
        let tmp = tempdir().unwrap();
        let cache = DownloadCache::new(tmp.path());
        let client = DownloadClient::new(cache, 4);
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_cache_check() {
        let tmp = tempdir().unwrap();
        let cache = DownloadCache::new(tmp.path());

        // Store a fake bottle
        let data = b"test bottle content";
        let hash = sha256_bytes(data);
        cache.store_bottle("test", "1.0.0", "x86_64_linux", data).unwrap();

        let client = DownloadClient::new(cache, 4).unwrap();

        // Should find it in cache
        assert!(client.cache().has_bottle("test", "1.0.0", "x86_64_linux"));
    }
}
