//! Vulnerability auditing for brewx packages
//!
//! This crate provides vulnerability scanning capabilities for brewx,
//! querying a pre-built vulnerability index to find known security issues
//! in installed packages.

mod database;
mod error;
mod report;
mod version;

pub use database::{VulnDatabase, VulnDatabaseConfig};
pub use error::{Error, Result};
pub use report::{AuditReport, Finding, Severity};
pub use version::version_affected;
