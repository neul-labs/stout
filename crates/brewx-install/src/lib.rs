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

pub use error::{Error, Result};
pub use extract::extract_bottle;
pub use link::{link_package, unlink_package};
pub use receipt::{InstallReceipt, write_receipt};

use std::path::Path;
