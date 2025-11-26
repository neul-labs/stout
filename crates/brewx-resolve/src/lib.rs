//! brewx-resolve: Dependency resolution for brewx
//!
//! This crate handles:
//! - Building dependency graphs
//! - Topological sorting for install order
//! - Conflict detection

mod error;
mod graph;
mod plan;

pub use error::{Error, Result};
pub use graph::DependencyGraph;
pub use plan::{InstallPlan, InstallStep};

use brewx_index::{Database, Formula, FormulaInfo};
