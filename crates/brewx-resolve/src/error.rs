//! Error types for brewx-resolve

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Index error: {0}")]
    Index(#[from] brewx_index::Error),

    #[error("Dependency cycle detected: {0}")]
    CycleDetected(String),

    #[error("Unresolved dependency: {0} requires {1}")]
    UnresolvedDependency(String, String),

    #[error("Conflict: {0} conflicts with {1}")]
    Conflict(String, String),
}

pub type Result<T> = std::result::Result<T, Error>;
