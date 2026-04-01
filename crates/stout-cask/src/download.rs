//! Cask artifact downloading

use crate::error::{Error, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tracing::debug;

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
        .user_agent(format!("stout/{}", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(30))
        .timeout(std::time::Duration::from_secs(600)) // 10 min total
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

    // Throttle progress bar redraws to ~10 fps to avoid screen flashing.
    let mut last_draw = std::time::Instant::now();
    const DRAW_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;

    while let Some(chunk) = tokio::time::timeout(std::time::Duration::from_secs(120), stream.next())
        .await
        .map_err(|_| {
            Error::DownloadFailed(format!(
                "download timed out for {} (no data for 120s)",
                token
            ))
        })?
    {
        let chunk = chunk.map_err(Error::Http)?;
        file.write_all(&chunk).await?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if last_draw.elapsed() >= DRAW_INTERVAL {
            pb.set_position(downloaded);
            last_draw = std::time::Instant::now();
        }
    }
    pb.set_position(downloaded); // ensure final value is shown

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

    debug!("Downloaded {} to {}", token, dest_path.display());
    Ok(dest_path)
}

/// Detect artifact type from a downloaded file's magic bytes.
///
/// URL-based detection can be wrong when the URL has no file extension (e.g.
/// `download?build=arm`) or hides the extension in a query parameter (e.g.
/// `rotation.php?file=HandBrake.dmg`). Reading the first four bytes of the
/// actual file gives a reliable answer.
///
/// Returns `None` only if the file cannot be read.
pub fn detect_artifact_type_from_magic(path: &Path) -> Option<ArtifactType> {
    use std::io::Read;
    let mut f = std::fs::File::open(path).ok()?;
    let mut buf = [0u8; 4];
    let n = f.read(&mut buf).ok()?;
    if n < 4 {
        return None;
    }
    Some(match buf {
        // ZIP: PK\x03\x04
        [0x50, 0x4B, 0x03, 0x04] => ArtifactType::Zip,
        // XAR (used by macOS .pkg): xar!
        [0x78, 0x61, 0x72, 0x21] => ArtifactType::Pkg,
        // gzip (tar.gz)
        [0x1F, 0x8B, _, _] => ArtifactType::TarGz,
        // Everything else for macOS cask downloads is a DMG:
        // zlib-compressed (78 xx), bzip2-compressed (42 5A 68), raw UDIF, etc.
        _ => ArtifactType::Dmg,
    })
}
