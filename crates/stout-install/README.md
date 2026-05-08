# stout-install

[![Crates.io](https://img.shields.io/crates/v/stout-install)](https://crates.io/crates/stout-install)
[![Docs.rs](https://docs.rs/stout-install/badge.svg)](https://docs.rs/stout-install)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Fast, reliable package installation for Homebrew-compatible packages — bottles, source builds, symlinks, and INSTALL_RECEIPT generation.

**Keywords:** package-installation, homebrew, bottle, source-build, symlink, cellar, rust, package-manager, formula

## Why stout-install?

Installing packages means more than just downloading a file. You need to extract archives, verify integrity, handle dependencies, create symlinks, and record what was installed so it can be updated or removed later. `stout-install` handles all of this with a clean async API, producing Homebrew-compatible installations that work alongside existing `brew` setups.

This crate powers the `stout install`, `stout uninstall`, and `stout upgrade` commands, but it's designed as a reusable library for any Rust project that needs to install pre-built binaries or build from source.

## Features

- **Bottle Installation** — Extract and install pre-built binaries (.tar.gz, .tar.zst)
- **Source Builds** — Build from source with configurable parallelism and compiler selection
- **Archive Extraction** — Handle tar.gz, tar.zst, zip, and other archive formats
- **Symlink Management** — Create and remove symlinks in the installation prefix
- **Homebrew Compatibility** — Generate `INSTALL_RECEIPT.json` files that brew understands
- **Dependency Installation** — Automatically install required dependencies first
- **Atomic Operations** — Install to a staging area, then atomically move into place
- **Rollback Support** — Restore previous versions on failed upgrades
- **Prefix Isolation** — Install to custom prefixes for multi-environment setups

## Installation

```bash
cargo add stout-install
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-install = "0.2"
```

## Quick Start

```rust
use stout_install::{Installer, InstallConfig};
use stout_index::Index;

// Open the index and create an installer
let index = Index::open_default()?;
let config = InstallConfig {
    cellar: "/opt/homebrew/Cellar".into(),
    prefix: "/opt/homebrew".into(),
    parallel_jobs: num_cpus::get(),
};
let installer = Installer::new(config, &index)?;

// Install a package (resolves dependencies, downloads bottle, extracts, links)
let result = installer.install("jq").await?;
println!("Installed {} to {}", result.name, result.path.display());
```

## API Overview

### Creating an Installer

```rust
use stout_install::{Installer, InstallConfig};

// Default configuration (uses system Homebrew paths)
let config = InstallConfig::default();
let installer = Installer::new(config, &index)?;

// Custom prefix (isolated environment)
let config = InstallConfig {
    cellar: "/myproject/.stout/Cellar".into(),
    prefix: "/myproject/.stout".into(),
    parallel_jobs: 8,
};
let installer = Installer::new(config, &index)?;
```

### Installing Packages

```rust
use stout_install::InstallOptions;

// Basic install (uses bottle if available, falls back to source)
let result = installer.install("jq").await?;

// Force source build
let options = InstallOptions {
    build_from_source: true,
    ..Default::default()
};
let result = installer.install_with_options("wget", &options).await?;

// Install specific version
let options = InstallOptions {
    version: Some("3.2.1".to_string()),
    ..Default::default()
};
let result = installer.install_with_options("fish", &options).await?;

// Install without dependencies (use with caution)
let options = InstallOptions {
    ignore_dependencies: true,
    ..Default::default()
};
let result = installer.install_with_options("node", &options).await?;
```

### Batch Installation

```rust
// Install multiple packages, resolving dependencies across the entire set
let results = installer.install_batch(&["jq", "curl", "wget"]).await?;

for result in results {
    println!("Installed: {} {}", result.name, result.version);
}
```

### Uninstalling Packages

```rust
// Remove a package and its symlinks
installer.uninstall("jq").await?;

// Uninstall but keep the downloaded archive (for quick reinstall)
installer.uninstall_keep_cache("wget").await?;

// Uninstall and remove unused dependencies
installer.uninstall_with_autoremove("node").await?;
```

### Upgrading Packages

```rust
// Upgrade a single package to the latest version
let result = installer.upgrade("jq").await?;
println!("Upgraded {} → {}", result.old_version, result.new_version);

// Upgrade all outdated packages
let results = installer.upgrade_all().await?;
for result in results {
    println!("Upgraded: {} {} → {}", result.name, result.old_version, result.new_version);
}

// Upgrade with rollback on failure
let result = installer.upgrade_with_rollback("postgresql").await?;
```

### Symlink Management

```rust
// Link an installed package to the prefix
installer.link("jq").await?;

// Unlink (remove symlinks but keep installed)
installer.unlink("jq").await?;

// Relink (useful after prefix changes)
installer.relink("jq").await?;

// Check linking status
let is_linked = installer.is_linked("jq")?;
```

### Pinning and Version Management

```rust
// Pin a package to prevent upgrades
installer.pin("python@3.11").await?;

// Unpin to allow upgrades again
installer.unpin("python@3.11").await?;

// Check if pinned
let is_pinned = installer.is_pinned("python@3.11")?;

// Switch between installed versions
installer.switch("python", "3.11").await?;
```

### Installation Inspection

```rust
// Get info about an installed package
let info = installer.installed_info("jq")?;
println!("Version: {}", info.version);
println!("Installed: {}", info.installed_at);
println!("Dependencies: {:?}", info.dependencies);
println!("Files: {}", info.file_count);

// List all installed packages
let installed = installer.list_installed()?;
for pkg in installed {
    println!("{} {}", pkg.name, pkg.version);
}

// Check if a package is installed
let is_installed = installer.is_installed("jq")?;

// Find which package owns a file
let owner = installer.owner_of("/opt/homebrew/bin/jq")?;
```

### Receipt Generation

Every installation generates a Homebrew-compatible `INSTALL_RECEIPT.json`:

```json
{
  "homebrew_version": "stout 0.2.1",
  "used_options": [],
  "unused_options": [],
  "built_as_bottle": true,
  "poured_from_bottle": true,
  "installed_as_dependency": true,
  "installed_on_request": false,
  "changed_files": [],
  "time": 1715000000,
  "source_modified_time": 0,
  "HEAD": null,
  "stdlib": null,
  "compiler": "clang",
  "aliases": [],
  "runtime_dependencies": [
    { "full_name": "oniguruma", "version": "6.9.9" }
  ],
  "source": {
    "path": "...",
    "tap": "homebrew/core",
    "spec": "stable",
    "versions": { "stable": "1.7.1", "head": null }
  }
}
```

## Performance

Installation performance is dominated by I/O:

| Operation | Typical Time | Notes |
|-----------|-------------|-------|
| Small bottle (jq, ~1MB) | 100-300ms | Download + extract + link |
| Medium bottle (node, ~50MB) | 2-5s | Mostly extraction time |
| Large bottle (rust, ~200MB) | 10-20s | Extraction of many files |
| Source build (small) | 30s-2min | Depends on build system |
| Source build (large) | 5-30min | Parallel compilation helps |

Atomic staging means installations are all-or-nothing — you'll never have a partially-installed package.

## Integration with the Stout Ecosystem

`stout-install` is the execution engine of stout's package management:

- **stout-index** provides package metadata (URLs, checksums, dependencies)
- **stout-resolve** computes the installation order
- **stout-fetch** downloads bottles and source tarballs
- **stout-state** tracks what's installed, pinned, and linked
- **stout-cask** extends installation to macOS applications

You can use `stout-install` standalone if you have your own source of package metadata, or combine it with the full stout stack for a complete Homebrew-compatible solution.

## License

MIT License — see the [repository root](../../LICENSE) for details.
