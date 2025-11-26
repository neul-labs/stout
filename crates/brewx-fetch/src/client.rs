//! HTTP client for downloading bottles

use crate::cache::DownloadCache;
use crate::error::{Error, Result};
use crate::progress::ProgressReporter;
use crate::verify::sha256_bytes;
use reqwest::Client;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Download client for bottles
pub struct DownloadClient {
    client: Client,
    cache: DownloadCache,
}

impl DownloadClient {
    pub fn new(cache: DownloadCache) -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("brewx/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        Ok(Self { client, cache })
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

    /// Download multiple bottles in parallel
    pub async fn download_bottles(
        &self,
        bottles: Vec<BottleSpec>,
        progress: &ProgressReporter,
    ) -> Result<Vec<PathBuf>> {
        use futures::future::join_all;

        let summary = progress.new_summary(bottles.len() as u64, "Downloading packages...");

        let futures: Vec<_> = bottles
            .into_iter()
            .map(|spec| {
                let client = self.client.clone();
                let cache = self.cache.bottle_path(&spec.name, &spec.version, &spec.platform);
                async move {
                    let result = self
                        .download_bottle(
                            &spec.name,
                            &spec.version,
                            &spec.platform,
                            &spec.url,
                            &spec.sha256,
                            Some(progress),
                        )
                        .await;
                    summary.inc(1);
                    result
                }
            })
            .collect();

        let results = join_all(futures).await;
        summary.finish();

        results.into_iter().collect()
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
