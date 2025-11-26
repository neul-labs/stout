//! brewx-fetch: Download management for brewx
//!
//! This crate handles:
//! - Parallel bottle downloads
//! - Checksum verification
//! - Download progress reporting
//! - Local download cache

mod cache;
mod client;
mod error;
mod progress;
mod verify;

#[cfg(test)]
mod tests;

pub use cache::DownloadCache;
pub use client::{BottleSpec, DownloadClient};
pub use error::{Error, Result};
pub use progress::{DownloadProgress, ProgressReporter};
pub use verify::verify_sha256;
