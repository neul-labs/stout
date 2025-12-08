//! Error types for stout-bundle

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Brewfile not found: {0}")]
    BrewfileNotFound(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Ruby parse error: {0}")]
    RubyError(String),

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
