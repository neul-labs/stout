//! stout-index: SQLite index management for stout
//!
//! This crate handles:
//! - Local SQLite database operations
//! - Index synchronization from remote
//! - Formula and cask search and lookup
//! - On-demand formula/cask JSON fetching
//! - Ed25519 signature verification
//! - Delta sync optimization

mod cask;
mod db;
mod delta;
mod error;
mod formula;
mod query;
mod schema;
mod signature;
mod sync;

#[cfg(test)]
mod tests;

pub use cask::{Cask, CaskInfo};
pub use db::Database;
pub use delta::{DeltaManifest, DeltaSync, SyncMetadata, SyncStats, UpdateStatus};
pub use error::{Error, Result};
pub use formula::{Bottle, Dependency, DependencyType, Formula, FormulaInfo};
pub use query::Query;
pub use signature::{
    compute_file_sha256, compute_sha256, SignatureVerifier, SignedManifest, TrustedKeys,
    VerificationResult, DEFAULT_PUBLIC_KEY_HEX,
};
pub use sync::{IndexSync, Manifest, SecurityPolicy};

/// Base URL for the stout-index repository
pub const DEFAULT_INDEX_URL: &str = "https://raw.githubusercontent.com/neul-labs/stout-index/main";
