//! Error types for stout-cask

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Cask not found: {0}")]
    CaskNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("Installation failed: {0}")]
    InstallFailed(String),

    #[error("Uninstall failed: {0}")]
    UninstallFailed(String),

    #[error("Mount failed: {0}")]
    MountFailed(String),

    #[error("Artifact not found: {0}")]
    ArtifactNotFound(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Command failed: {cmd} - {message}")]
    CommandFailed { cmd: String, message: String },

    #[error("State error: {0}")]
    State(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;
