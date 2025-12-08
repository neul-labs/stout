//! stout-install: Package installation for stout
//!
//! This crate handles:
//! - Extracting bottles to Cellar
//! - Building from source when bottles unavailable
//! - Creating bottles from installed packages
//! - Creating symlinks
//! - Writing INSTALL_RECEIPT.json
//! - Running post-install hooks
//! - Parallel installation support

mod bottle;
mod build;
mod error;
mod extract;
mod link;
mod parallel;
mod receipt;

#[cfg(test)]
mod tests;

pub use bottle::{create_bottle, BottleResult};
pub use build::{BuildConfig, BuildResult, SourceBuilder, can_build_from_source};
pub use error::{Error, Result};
pub use extract::{extract_bottle, remove_package};
pub use link::{link_package, unlink_package};
pub use parallel::{BottleInfo, LinkInfo, PackageInstallResult, ParallelConfig, ParallelInstaller};
pub use receipt::{InstallReceipt, RuntimeDependency, write_receipt};
