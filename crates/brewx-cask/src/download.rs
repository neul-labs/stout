//! Cask artifact downloading

use crate::error::{Error, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info};

/// Artifact type detected from URL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactType {
    Dmg,
    Pkg,
    Zip,
    TarGz,
    TarBz2,
    AppImage,
}

impl ArtifactType {
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Dmg => "dmg",
            Self::Pkg => "pkg",
            Self::Zip => "zip",
            Self::TarGz => "tar.gz",
            Self::TarBz2 => "tar.bz2",
            Self::AppImage => "AppImage",
        }
    }
}

/// Download a cask artifact
pub async fn download_cask_artifact(
    url: &str,
    dest_dir: &Path,
    token: &str,
    expected_sha256: Option<&str>,
    artifact_type: ArtifactType,
) -> Result<PathBuf> {
    let client = Client::builder()
        .user_agent("brewx/0.1")
        .build()
        .map_err(Error::Http)?;

    let filename = format!("{}.{}", token, artifact_type.extension());
    let dest_path = dest_dir.join(&filename);

    debug!("Downloading {} to {}", url, dest_path.display());

    // Start download
    let response = client.get(url).send().await.map_err(Error::Http)?;

    if !response.status().is_success() {
        return Err(Error::DownloadFailed(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    let total_size = response.content_length().unwrap_or(0);

    // Create progress bar
    let pb = ProgressBar::new(total_size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Ensure parent directory exists
    if let Some(parent) = dest_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Download with progress
    let mut file = File::create(&dest_path).await?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0u64;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(Error::Http)?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        pb.set_position(downloaded);
    }

    pb.finish_and_clear();
    file.flush().await?;

    // Verify checksum if provided
    if let Some(expected) = expected_sha256 {
        if expected != "no_check" {
            let actual = hex::encode(hasher.finalize());
            if actual != expected {
                // Remove the corrupted file
                let _ = tokio::fs::remove_file(&dest_path).await;
                return Err(Error::ChecksumMismatch {
                    expected: expected.to_string(),
                    actual,
                });
            }
            debug!("Checksum verified: {}", actual);
        }
    }

    info!("Downloaded {} to {}", token, dest_path.display());
    Ok(dest_path)
}
