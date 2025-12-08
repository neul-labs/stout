//! stout-bundle: Brewfile parsing and bundle management
//!
//! This crate provides:
//! - Brewfile parsing (Ruby DSL compatibility)
//! - Bundle install/check/cleanup operations
//! - Brewfile generation from installed packages
//! - Snapshot creation and restoration

mod error;
mod parser;
mod snapshot;

pub use error::{Error, Result};
pub use parser::{Brewfile, BrewEntry, CaskEntry, TapEntry, MasEntry};
pub use snapshot::{Snapshot, SnapshotManager};
