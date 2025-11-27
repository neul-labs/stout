//! Error types for brewx-install

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Link failed: {0}")]
    LinkFailed(String),

    #[error("Invalid bottle format: {0}")]
    InvalidBottle(String),

    #[error("Build failed: {0}")]
    Build(String),

    #[error("Bottle creation failed: {0}")]
    Bottle(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
