//! brewx-resolve: Dependency resolution for brewx
//!
//! This crate handles:
//! - Building dependency graphs
//! - Topological sorting for install order
//! - Conflict detection

mod error;
mod graph;
mod plan;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use graph::DependencyGraph;
pub use plan::{InstallPlan, InstallStep};
