# stout-mirror

[![Crates.io](https://img.shields.io/crates/v/stout-mirror)](https://crates.io/crates/stout-mirror)
[![Docs.rs](https://docs.rs/stout-mirror/badge.svg)](https://docs.rs/stout-mirror)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Offline mirror creation and HTTP serving for air-gapped environments — replicate package repositories for environments without internet access.

**Keywords:** offline-mirror, air-gapped, package-mirror, http-server, package-manager, enterprise, rust, homebrew, reproducible-builds

## Why stout-mirror?

CI/CD pipelines, secure environments, and remote sites often can't reach the internet. `stout-mirror` lets you create a self-contained copy of everything needed to install a set of packages — formulas, casks, bottles, and the index — then serve it locally via HTTP so stout (or other tools) can install from it.

This crate powers the `stout mirror` commands but is designed as a reusable library for any Rust project that needs to create offline package repositories.

## Features

- **Offline Mirror Creation** — Download all packages and dependencies to a local directory
- **Built-in HTTP Server** — Serve mirrors locally with a single command
- **Transitive Dependency Resolution** — Automatically include all required dependencies
- **Formula & Cask Support** — Mirror both CLI tools and applications
- **Integrity Verification** — Verify all downloads with SHA256 checksums
- **Incremental Updates** — Add packages to an existing mirror without re-downloading everything
- **Compressed Storage** — Store bottles in their original compressed form
- **Mirror Verification** — Check that a mirror is complete and uncorrupted
- **Custom Base URL** — Support for serving behind reverse proxies or on different ports

## Installation

```bash
cargo add stout-mirror
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-mirror = "0.2"
```

## Quick Start

```rust
use stout_mirror::Mirror;
use stout_index::Index;

let index = Index::open_default()?;

// Create a mirror with specific packages
let mirror = Mirror::create("/path/to/mirror", &index)
    .with_packages(&["jq", "curl", "wget", "node"])
    .build()
    .await?;

println!("Mirror created: {} packages", mirror.package_count());

// Serve the mirror
mirror.serve("0.0.0.0:8080").await?;
```

## API Overview

### Creating Mirrors

```rust
use stout_mirror::{Mirror, MirrorBuilder};

// Simple mirror with explicit packages
let mirror = Mirror::create("/path/to/mirror", &index)
    .with_packages(&["jq", "curl"])
    .build()
    .await?;

// Mirror from a Brewfile
let mirror = Mirror::create("/path/to/mirror", &index)
    .with_brewfile("Brewfile")
    .build()
    .await?;

// Mirror all installed packages
let mirror = Mirror::create("/path/to/mirror", &index)
    .with_installed()
    .build()
    .await?;

// Full configuration
let mirror = Mirror::create("/path/to/mirror", &index)
    .with_packages(&["postgresql@16"])
    .with_casks(&["docker"])
    .include_source_tarballs(false)  // Only bottles (faster, smaller)
    .include_build_deps(true)       // Include build dependencies
    .compression(Compression::Zstd)   // Recompress with zstd
    .build()
    .await?;
```

### Serving Mirrors

```rust
use stout_mirror::MirrorServer;

// Built-in HTTP server
let server = MirrorServer::new(&mirror);
server.bind("0.0.0.0:8080").await?;

// With custom configuration
let server = MirrorServer::new(&mirror)
    .with_workers(4)
    .with_log_level(tracing::Level::INFO);
server.bind("127.0.0.1:9000").await?;

// The mirror exposes:
// GET /index.json          — Package index
// GET /bottles/<pkg>.tar.gz — Bottle downloads
// GET /casks/<pkg>.dmg      — Cask downloads
// GET /api/health          — Health check
```

### Incremental Updates

```rust
// Add packages to an existing mirror
let mirror = Mirror::open("/path/to/mirror")?;
mirror.add_packages(&["python@3.12", "rust"]).await?;

// Remove packages
mirror.remove_packages(&["old-package"]).await?;

// Sync with latest index
mirror.update_index().await?;
```

### Verification

```rust
// Verify mirror integrity
let report = mirror.verify().await?;

println!("Packages: {}", report.package_count);
println!("Missing files: {}", report.missing.len());
println!("Corrupt files: {}", report.corrupt.len());

if !report.is_valid() {
    println!("Mirror has issues — run repair to fix");
    mirror.repair().await?;
}
```

### Mirror Inspection

```rust
// List mirrored packages
let packages = mirror.list_packages()?;
for pkg in packages {
    println!("{} {} — {}", pkg.name, pkg.version, pkg.size);
}

// Get mirror size
let size = mirror.total_size()?;
println!("Mirror size: {}", humansize::format_size(size));

// Get mirror info
let info = mirror.info()?;
println!("Created: {}", info.created_at);
println!( "Index version: {}", info.index_version);
println!("Packages: {}", info.package_count);
```

### Using a Mirror

Configure stout (or your application) to use the mirror:

```toml
# ~/.stout/config.toml
[index]
base_url = "http://localhost:8080"
```

Or programmatically:

```rust
use stout_index::Index;

let index = Index::open_default()?;
index.set_mirror_url("http://localhost:8080")?;
index.update().await?; // Will fetch from mirror instead of upstream
```

### Export and Transport

```rust
// Create a tarball of the mirror for transport
mirror.export_tar("/tmp/mirror.tar.gz").await?;

// Import on another machine
let mirror = Mirror::import_tar("/tmp/mirror.tar.gz", "/var/lib/stout/mirror").await?;

// Or use rsync/robocopy — mirrors are just directories
```

## Enterprise Use Cases

### CI/CD Caching

```rust
// Pre-build a mirror with your CI dependencies
Mirror::create("./ci-mirror", &index)
    .with_brewfile("Brewfile")
    .build()
    .await?;

// Cache ./ci-mirror in your CI system
// Subsequent builds use the cached mirror
```

### Air-Gapped Networks

```rust
// On internet-connected machine
Mirror::create("/media/usb/stout-mirror", &index)
    .with_installed()
    .build()
    .await?;

// Transfer /media/usb to air-gapped network
// Serve from the USB or copy to internal server
```

### Internal Package Hosting

```rust
// Create mirror on internal server
let mirror = Mirror::create("/var/lib/stout/mirror", &index)
    .with_packages(&["internal-tool", "jq", "curl"])
    .build()
    .await?;

// Serve behind nginx/Apache reverse proxy
mirror.serve("127.0.0.1:9000").await?;
```

## Performance

Mirror creation is I/O and network bound:

| Scenario | Size | Time (100Mbps) |
|----------|------|----------------|
| Small workspace (5-10 packages) | 50-200MB | 1-5 min |
| Developer machine (50 packages) | 500MB-2GB | 5-20 min |
| Full CI environment (100+ packages) | 2-10GB | 20-60 min |

Incremental updates only download changed packages. The built-in HTTP server handles 1000+ concurrent requests on modest hardware.

## Integration with the Stout Ecosystem

`stout-mirror` is the offline/enterprise layer of stout:

- **stout-index** provides package metadata and URLs to download
- **stout-resolve** computes the full set of packages to mirror
- **stout-fetch** downloads bottles and archives into the mirror
- **stout-install** can install from a mirror instead of upstream
- **stout-bundle** lets you define what goes into a mirror declaratively

You can use `stout-mirror` standalone for any project that needs to create offline package repositories.

## License

MIT License — see the [repository root](../../LICENSE) for details.
