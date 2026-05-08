# stout-fetch

[![Crates.io](https://img.shields.io/crates/v/stout-fetch)](https://crates.io/crates/stout-fetch)
[![Docs.rs](https://docs.rs/stout-fetch/badge.svg)](https://docs.rs/stout-fetch)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Async download manager with progress bars, parallel transfers, SHA256 verification, and resume support for Rust applications.

**Keywords:** download-manager, async, http-client, progress-bar, sha256, cache, rust, tokio, parallel-downloads

## Why stout-fetch?

Downloading files in CLI tools usually means either blocking the main thread with `reqwest` or writing a lot of boilerplate for progress bars, retries, and checksums. `stout-fetch` provides a production-ready downloader that handles all of this with a simple API.

This crate powers the `stout install`, `stout update`, and `stout mirror` commands, but it's designed as a general-purpose download library for any Rust CLI or service that needs reliable, user-friendly file transfers.

## Features

- **Async HTTP Downloads** — Built on `reqwest` with `tokio` for non-blocking I/O
- **Progress Bar Integration** — Beautiful progress bars via `indicatif` with ETA and throughput
- **SHA256 Checksum Verification** — Automatic integrity verification after download
- **Parallel Downloads** — Download multiple files concurrently with configurable limits
- **Resume Support** — Resume interrupted downloads using HTTP Range requests
- **Disk Caching** — TTL-based cache with automatic cleanup and size limits
- **Retry Logic** — Exponential backoff with configurable max retries
- **Streaming** — Stream large files without loading into memory
- **Compression** — Automatic gzip decompression support

## Installation

```bash
cargo add stout-fetch
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-fetch = "0.2"
```

## Quick Start

```rust
use stout_fetch::Downloader;

// Create a downloader with default settings
let downloader = Downloader::new("~/.cache/downloads")?;

// Download a file with automatic checksum verification
let path = downloader
    .download("https://example.com/file.tar.gz")
    .with_sha256("a1b2c3...")
    .fetch()
    .await?;

println!("Downloaded to: {}", path.display());
```

## API Overview

### Creating a Downloader

```rust
use stout_fetch::{Downloader, CacheConfig};

// Simple setup with default cache
let downloader = Downloader::new("~/.cache/myapp")?;

// With custom cache configuration
let config = CacheConfig {
    max_size: 1024 * 1024 * 1024, // 1GB
    ttl_seconds: 86400,            // 24 hours
    cleanup_interval: 3600,         // 1 hour
};
let downloader = Downloader::with_config("~/.cache/myapp", config)?;

// Ephemeral (no cache)
let downloader = Downloader::ephemeral()?;
```

### Single File Download

```rust
use stout_fetch::DownloadRequest;

// Basic download
let path = downloader
    .download("https://example.com/data.json")
    .fetch()
    .await?;

// With SHA256 verification
let path = downloader
    .download("https://example.com/file.tar.gz")
    .with_sha256("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    .fetch()
    .await?;

// With progress bar label
let path = downloader
    .download("https://example.com/large-file.zip")
    .with_label("Downloading large-file.zip")
    .with_sha256("...")
    .fetch()
    .await?;

// With custom timeout
let path = downloader
    .download("https://example.com/slow-server/file")
    .with_timeout(std::time::Duration::from_secs(120))
    .fetch()
    .await?;
```

### Batch / Parallel Downloads

```rust
use stout_fetch::BatchDownload;

let requests = vec![
    BatchDownload::new("https://example.com/a.tar.gz")
        .with_sha256("...")
        .with_label("Package A"),
    BatchDownload::new("https://example.com/b.tar.gz")
        .with_sha256("...")
        .with_label("Package B"),
    BatchDownload::new("https://example.com/c.tar.gz")
        .with_sha256("...")
        .with_label("Package C"),
];

// Download up to 4 files concurrently
let results = downloader
    .download_batch(requests)
    .with_concurrency(4)
    .fetch()
    .await?;

for result in results {
    match result {
        Ok(path) => println!("Downloaded: {}", path.display()),
        Err(e) => eprintln!("Failed: {}", e),
    }
}
```

### Streaming Downloads

```rust
use tokio::io::AsyncWriteExt;

// Stream directly to a writer without caching to disk first
let mut stream = downloader
    .download("https://example.com/stream.bin")
    .stream()
    .await?;

let mut file = tokio::fs::File::create("output.bin").await?;
tokio::io::copy(&mut stream, &mut file).await?;
```

### Resume Support

```rust
// Downloads automatically resume if the partial file exists
// and the server supports Range requests
let path = downloader
    .download("https://example.com/huge-file.iso")
    .with_resume(true)
    .fetch()
    .await?;
```

### Cache Management

```rust
// Check if a URL is already cached and valid
if downloader.is_cached(url, Some(expected_sha256))? {
    let path = downloader.cache_path(url)?;
    println!("Using cached file: {}", path.display());
} else {
    let path = downloader.download(url).with_sha256(expected_sha256).fetch().await?;
}

// Manually purge expired cache entries
let cleaned = downloader.cleanup_cache()?;
println!("Removed {} expired cache entries", cleaned);

// Get cache statistics
let stats = downloader.cache_stats()?;
println!("Cache size: {} / {}", stats.used, stats.max);
```

### Progress Bar Customization

```rust
use stout_fetch::{Downloader, ProgressStyle};

let downloader = Downloader::new("~/.cache")?
    .with_progress_style(ProgressStyle::detailed());

// Or per-download
let path = downloader
    .download(url)
    .with_progress_style(ProgressStyle::minimal())
    .fetch()
    .await?;
```

## Performance

Typical download performance on a 100Mbps connection:

| Scenario | Throughput | Notes |
|----------|-----------|-------|
| Single small file (<1MB) | Instant | Cached after first download |
| Single large file (100MB) | ~90Mbps | Single-threaded, saturates connection |
| 4 parallel files | ~95Mbps | Near line-rate with concurrency |
| 16 parallel files | ~95Mbps | Diminishing returns beyond 8-12 |
| Resumed download | Varies | Skips already-downloaded bytes |

The downloader uses a connection pool behind the scenes, so repeated requests to the same host reuse TCP connections.

## Error Handling

```rust
use stout_fetch::DownloadError;

match downloader.download(url).fetch().await {
    Ok(path) => { /* success */ }
    Err(DownloadError::ChecksumMismatch { expected, actual }) => {
        eprintln!("Corrupted download! Expected {}, got {}", expected, actual);
    }
    Err(DownloadError::HttpStatus(status)) => {
        eprintln!("Server returned {}", status);
    }
    Err(DownloadError::Io(e)) => {
        eprintln!("Disk error: {}", e);
    }
    Err(e) => eprintln!("Download failed: {}", e),
}
```

## Integration with the Stout Ecosystem

`stout-fetch` is the network layer of stout:

- **stout-index** uses it to download index updates
- **stout-install** uses it to fetch bottles and source tarballs
- **stout-mirror** uses it to bulk-download packages for offline use
- **stout-cask** uses it to download application installers

You can use `stout-fetch` standalone for any project that needs reliable async downloads with progress reporting.

## License

MIT License — see the [repository root](../../LICENSE) for details.
