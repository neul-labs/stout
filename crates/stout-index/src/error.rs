//! Error types for stout-index

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

    #[error("Cask not found: {0}")]
    CaskNotFound(String),

    #[error("Index not initialized. Run 'stout update' first.")]
    IndexNotInitialized,

    #[error("Checksum mismatch for {0}")]
    ChecksumMismatch(String),

    #[error("Invalid index format: {0}")]
    InvalidIndex(String),

    #[error("Signature verification failed: {0}")]
    SignatureInvalid(String),

    #[error("Signature missing: index manifest is not signed")]
    SignatureMissing,

    #[error("Signature expired: signed {0} seconds ago (max allowed: {1})")]
    SignatureExpired(u64, u64),

    #[error("Untrusted key: the signing key is not in the trusted keys list")]
    UntrustedKey,

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;
