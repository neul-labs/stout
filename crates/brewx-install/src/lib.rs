//! brewx-install: Package installation for brewx
//!
//! This crate handles:
//! - Extracting bottles to Cellar
//! - Creating symlinks
//! - Writing INSTALL_RECEIPT.json
//! - Running post-install hooks

mod error;
mod extract;
mod link;
mod receipt;

#[cfg(test)]
mod tests;

pub use error::{Error, Result};
pub use extract::{extract_bottle, remove_package};
pub use link::{link_package, unlink_package};
pub use receipt::{InstallReceipt, RuntimeDependency, write_receipt};
