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
pub mod cask_scan;
pub mod cellar;
mod error;
mod extract;
mod link;
mod parallel;
mod receipt;

#[cfg(test)]
mod tests;

pub use bottle::{create_bottle, BottleResult};
pub use build::{can_build_from_source, BuildConfig, BuildResult, SourceBuilder};
pub use cask_scan::{count_caskroom_casks, scan_caskroom, InstalledBrewCask};
pub use cellar::{
    count_cellar_packages, parse_brew_receipt, scan_cellar, scan_cellar_package, timestamp_to_iso,
    BrewReceipt, BrewRuntimeDep, CellarPackage,
};
pub use error::{BuildError, Error, Result};
pub use extract::{
    extract_bottle, relocate_bottle, remove_package, scan_cellar_unrelocated,
    scan_unrelocated_files,
};
pub use link::{link_package, unlink_package};
pub use parallel::{BottleInfo, LinkInfo, PackageInstallResult, ParallelConfig, ParallelInstaller};
pub use receipt::{write_receipt, InstallReceipt, RuntimeDependency};
