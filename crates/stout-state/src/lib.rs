//! stout-state: Local state management for stout
//!
//! This crate handles:
//! - User configuration (config.toml)
//! - Installed packages tracking (installed.toml)
//! - Tap management (taps.toml)
//! - Lockfile support (stout.lock)
//! - Package history tracking (history.json)
//! - Directory paths and defaults

mod config;
mod error;
mod history;
mod installed;
mod lockfile;
mod paths;
mod tap;

#[cfg(test)]
mod tests;

pub use config::Config;
pub use error::{Error, Result};
pub use history::{HistoryAction, HistoryEntry, PackageHistory};
pub use installed::{InstalledPackage, InstalledPackages};
pub use lockfile::{LockedPackage, Lockfile};
pub use paths::Paths;
pub use tap::{Tap, TapManager};
