//! Error types for stout-mirror

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Mirror directory not found: {0}")]
    MirrorNotFound(PathBuf),

    #[error("Invalid mirror: missing manifest.json")]
    InvalidMirror,

    #[error("Package not found in mirror: {0}")]
    PackageNotFound(String),

    #[error("Platform not available in mirror: {0} for {1}")]
    PlatformNotAvailable(String, String),

    #[error("Failed to create mirror directory: {0}")]
    CreateDir(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Checksum mismatch for {0}: expected {1}, got {2}")]
    ChecksumMismatch(String, String, String),

    #[error("Index error: {0}")]
    Index(#[from] stout_index::Error),

    #[error("Mirror manifest error: {0}")]
    Manifest(String),

    #[error("Server error: {0}")]
    Server(String),
}

pub type Result<T> = std::result::Result<T, Error>;
