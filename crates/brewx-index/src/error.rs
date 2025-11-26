//! Error types for brewx-index

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    #[error("Index not initialized. Run 'brewx update' first.")]
    IndexNotInitialized,

    #[error("Checksum mismatch for {0}")]
    ChecksumMismatch(String),

    #[error("Invalid index format: {0}")]
    InvalidIndex(String),
}

pub type Result<T> = std::result::Result<T, Error>;
