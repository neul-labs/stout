//! HTTP client for downloading bottles

use crate::cache::DownloadCache;
use crate::error::{Error, Result};
use crate::progress::ProgressReporter;
use crate::verify::sha256_bytes;
use futures::future::join_all;
use reqwest::Client;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info};

/// GitHub Container Registry API v2 URL prefix for Homebrew bottles
const GHCR_V2_URL_PREFIX: &str = "https://ghcr.io/v2/";

/// Download client for bottles
#[derive(Clone)]
pub struct DownloadClient {
    client: Client,
    cache: Arc<DownloadCache>,
    /// Semaphore to limit concurrent downloads
    semaphore: Arc<Semaphore>,
    /// Cache for OAuth tokens (scope -> token)
    token_cache: Arc<RwLock<HashMap<String, String>>>,
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
            token_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create with default concurrency (4 concurrent downloads)
    /// This balance between parallelism and avoiding network congestion
    pub fn with_cache(cache: DownloadCache) -> Result<Self> {
        Self::new(cache, 4)
    }

    /// Get a cached token or fetch a new one for ghcr.io
    async fn get_ghcr_token(&self, scope: &str) -> Result<String> {
        // Check cache first
        {
            let cache = self.token_cache.read().await;
            if let Some(token) = cache.get(scope) {
                return Ok(token.clone());
            }
        }

        // Fetch new token
        let token_url = format!(
            "https://ghcr.io/token?service=ghcr.io&scope={}",
            urlencoding::encode(scope)
        );

        debug!("Fetching ghcr.io token for scope: {}", scope);

        let response = self.client.get(&token_url).send().await?;

        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "Failed to get ghcr.io token: HTTP {}",
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct TokenResponse {
            token: String,
        }

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| Error::DownloadFailed(format!("Failed to parse token response: {}", e)))?;

        // Cache the token
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(scope.to_string(), token_resp.token.clone());
        }

        Ok(token_resp.token)
    }

    /// Extract the repository scope from a ghcr.io URL
    fn get_ghcr_scope(url: &str) -> Option<String> {
        // URL format: https://ghcr.io/v2/homebrew/core/PACKAGE/blobs/sha256:...
        // For versioned packages like openssl@3, the URL becomes:
        // https://ghcr.io/v2/homebrew/core/openssl/3/blobs/...
        if !url.starts_with(GHCR_V2_URL_PREFIX) {
            return None;
        }

        // Extract repository path: homebrew/core/PACKAGE or homebrew/core/PACKAGE/VERSION
        let path = url.strip_prefix(GHCR_V2_URL_PREFIX)?;
        let parts: Vec<&str> = path.split('/').collect();

        // Find the index of "blobs" or "manifests" to know where the repo path ends
        let end_idx = parts
            .iter()
            .position(|&p| p == "blobs" || p == "manifests")?;

        if end_idx >= 3 {
            // Join all parts up to (but not including) blobs/manifests
            let repo = parts[..end_idx].join("/");
            Some(format!("repository:{}:pull", repo))
        } else {
            None
        }
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
        if let Some(path) = self.cache.get_bottle(name, version, platform)? {
            // Verify cached file
            if crate::verify::verify_sha256(&path, expected_sha256).is_ok() {
                debug!("Using cached bottle for {}", name);
                return Ok(path);
            }
            // Cache corrupted, remove it
            self.cache.remove_bottle(name, version, platform)?;
        }

        // Download with authentication if needed
        debug!("Downloading {} from {}", name, url);

        // Check if this is a ghcr.io URL that needs authentication
        let response = if let Some(scope) = Self::get_ghcr_scope(url) {
            let token = self.get_ghcr_token(&scope).await?;
            self.client
                .get(url)
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?
        } else {
            self.client.get(url).send().await?
        };

        if !response.status().is_success() {
            return Err(Error::DownloadFailed(format!(
                "HTTP {} {}: {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown"),
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
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| Error::DownloadFailed("Semaphore closed".to_string()))?;

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
        let _hash = sha256_bytes(data);
        cache
            .store_bottle("test", "1.0.0", "x86_64_linux", data)
            .unwrap();

        let client = DownloadClient::new(cache, 4).unwrap();

        // Should find it in cache
        assert!(client.cache().has_bottle("test", "1.0.0", "x86_64_linux"));
    }

    // ========================================================================
    // get_ghcr_scope tests
    // ========================================================================

    #[test]
    fn test_get_ghcr_scope_simple_package() {
        // Simple package like wget
        let url = &format!(
            "{}homebrew/core/wget/blobs/sha256:abc123",
            GHCR_V2_URL_PREFIX
        );
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(
            scope,
            Some("repository:homebrew/core/wget:pull".to_string())
        );
    }

    #[test]
    fn test_get_ghcr_scope_versioned_package() {
        // Versioned package like openssl@3 -> openssl/3
        let url = &format!(
            "{}homebrew/core/openssl/3/blobs/sha256:abc123",
            GHCR_V2_URL_PREFIX
        );
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(
            scope,
            Some("repository:homebrew/core/openssl/3:pull".to_string())
        );
    }

    #[test]
    fn test_get_ghcr_scope_python_versioned() {
        // Python with version like python@3.14 -> python/3.14
        let url = &format!(
            "{}homebrew/core/python/3.14/blobs/sha256:abc123",
            GHCR_V2_URL_PREFIX
        );
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(
            scope,
            Some("repository:homebrew/core/python/3.14:pull".to_string())
        );
    }

    #[test]
    fn test_get_ghcr_scope_manifest_url() {
        // Manifest URL format
        let url = &format!("{}homebrew/core/wget/manifests/latest", GHCR_V2_URL_PREFIX);
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(
            scope,
            Some("repository:homebrew/core/wget:pull".to_string())
        );
    }

    #[test]
    fn test_get_ghcr_scope_non_ghcr_url() {
        // Non-ghcr.io URL should return None
        let url = "https://example.com/bottle.tar.gz";
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(scope, None);
    }

    #[test]
    fn test_get_ghcr_scope_too_short_path() {
        // Path too short (missing package name)
        let url = &format!("{}homebrew/core/blobs/sha256:abc123", GHCR_V2_URL_PREFIX);
        let scope = DownloadClient::get_ghcr_scope(url);
        assert_eq!(scope, None);
    }
}
