//! Error types for brewx-audit

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Vulnerability database not found at {0}")]
    DatabaseNotFound(PathBuf),

    #[error("Failed to open database: {0}")]
    DatabaseOpen(#[from] rusqlite::Error),

    #[error("Failed to decompress database: {0}")]
    Decompress(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    #[error("No vulnerability data available for formula: {0}")]
    NoVulnData(String),
}

pub type Result<T> = std::result::Result<T, Error>;
