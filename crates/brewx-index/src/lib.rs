//! brewx-index: SQLite index management for brewx
//!
//! This crate handles:
//! - Local SQLite database operations
//! - Index synchronization from remote
//! - Formula search and lookup
//! - On-demand formula JSON fetching

mod db;
mod error;
mod formula;
mod query;
mod schema;
mod sync;

#[cfg(test)]
mod tests;

pub use db::Database;
pub use error::{Error, Result};
pub use formula::{Bottle, Dependency, DependencyType, Formula, FormulaInfo};
pub use query::Query;
pub use sync::{IndexSync, Manifest};

/// Base URL for the brewx-index repository
pub const DEFAULT_INDEX_URL: &str = "https://raw.githubusercontent.com/anthropics/brewx-index/main";
