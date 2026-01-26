# stout-fetch

Download management for stout.

## Overview

This crate provides download functionality with progress tracking, parallel downloads, caching, and integrity verification.

## Features

- Async HTTP downloads with reqwest
- Progress bar integration with indicatif
- SHA256 checksum verification
- Download caching with TTL
- Parallel download support
- Resume interrupted downloads

## Usage

This crate is primarily used internally by other stout crates for fetching bottles, casks, and index data.

```rust
use stout_fetch::Downloader;

let downloader = Downloader::new(cache_dir)?;
let path = downloader.download(url, expected_sha256).await?;
```

## License

MIT License - see the repository root for details.
