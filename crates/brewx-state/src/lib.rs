//! brewx-state: Local state management for brewx
//!
//! This crate handles:
//! - User configuration (config.toml)
//! - Installed packages tracking (installed.toml)
//! - Directory paths and defaults

mod config;
mod error;
mod installed;
mod paths;

pub use config::Config;
pub use error::{Error, Result};
pub use installed::{InstalledPackage, InstalledPackages};
pub use paths::Paths;
