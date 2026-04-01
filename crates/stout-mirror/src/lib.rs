//! Offline mirror support for stout
//!
//! This crate provides functionality to:
//! - Create local mirrors with specific packages
//! - Serve mirrors via built-in HTTP server
//! - Use mirrors as package sources (client side)

mod client;
mod create;
mod error;
mod manifest;
mod serve;

pub use client::{MirrorClient, MirrorClientConfig};
pub use create::{create_mirror, detect_platform, MirrorConfig};
pub use error::{Error, Result};
pub use manifest::{BottleInfo, MirrorManifest, PackageInfo};
pub use serve::{serve_mirror, ServeConfig};
