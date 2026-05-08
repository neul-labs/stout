# stout-cask

[![Crates.io](https://img.shields.io/crates/v/stout-cask)](https://crates.io/crates/stout-cask)
[![Docs.rs](https://docs.rs/stout-cask/badge.svg)](https://docs.rs/stout-cask)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Application installation and management for macOS (.app, .pkg, .dmg) and Linux (AppImage, Flatpak) — the cask layer for Homebrew-compatible package managers.

**Keywords:** cask, application-installer, macos, dmg, pkg, appimage, flatpak, homebrew, rust, application-management

## Why stout-cask?

GUI applications aren't just binaries — they come as `.app` bundles, `.dmg` disk images, `.pkg` installers, and on Linux as AppImages or Flatpaks. `stout-cask` handles all of these formats with a unified API, managing the full lifecycle from download to uninstall, including macOS quarantine attributes and application symlinks.

This crate powers the `stout cask` commands but is designed as a reusable library for any Rust project that needs to install macOS or Linux applications programmatically.

## Features

- **macOS Application Support** — Install `.app` bundles from DMG, ZIP, and PKG archives
- **Linux Application Support** — Handle AppImage and Flatpak installations
- **Archive Extraction** — Mount DMG, extract ZIP, and run PKG installers automatically
- **Quarantine Management** — Remove macOS quarantine attributes (`com.apple.quarantine`)
- **Symlink Binaries** — Link application binaries into `/usr/local/bin` or custom prefix
- **Application Tracking** — Track installed casks with version and path metadata
- **Upgrade Support** — Replace old versions while preserving user data
- **Uninstall** — Clean removal including app bundles, symlinks, and preferences (optional)

## Installation

```bash
cargo add stout-cask
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-cask = "0.2"
```

## Quick Start

```rust
use stout_cask::CaskInstaller;
use stout_index::Index;

let index = Index::open_default()?;
let installer = CaskInstaller::new(&index)?;

// Install an application
let result = installer.install("visual-studio-code").await?;
println!("Installed {} to {}", result.name, result.app_path.display());

// Uninstall
installer.uninstall("visual-studio-code").await?;
```

## API Overview

### Creating an Installer

```rust
use stout_cask::{CaskInstaller, CaskConfig};

// Default configuration
let installer = CaskInstaller::new(&index)?;

// Custom install directory (default is /Applications on macOS)
let config = CaskConfig {
    appdir: "/Users/shared/Applications".into(),
    binarydir: "/Users/shared/bin".into(),
};
let installer = CaskInstaller::with_config(&index, config)?;
```

### Installing Applications

```rust
// Install by cask name
let result = installer.install("firefox").await?;
println!("App: {}", result.app_path.display());
println!("Version: {}", result.version);

// Install with options
let options = InstallOptions {
    force: true,              // Reinstall if already installed
    skip_quarantine: false,   // Remove quarantine attributes
    skip_checksum: false,     // Verify SHA256
};
let result = installer.install_with_options("slack", &options).await?;
```

### Uninstalling Applications

```rust
// Standard uninstall (removes app bundle and symlinks)
installer.uninstall("firefox").await?;

// Uninstall with options
let options = UninstallOptions {
    remove_preferences: true,   // Also remove user preferences
    remove_logs: true,        // Remove log files
    force: false,             // Ignore if not installed
};
installer.uninstall_with_options("slack", &options).await?;
```

### Upgrading Applications

```rust
// Upgrade a single cask
let result = installer.upgrade("firefox").await?;
println!("Upgraded {} → {}", result.old_version, result.new_version);

// Upgrade all outdated casks
let results = installer.upgrade_all().await?;
```

### Listing and Querying

```rust
// List installed casks
let installed = installer.list_installed().await?;
for cask in installed {
    println!("{} {} — {}", cask.name, cask.version, cask.app_path.display());
}

// Check if a cask is installed
let is_installed = installer.is_installed("firefox")?;

// Get info about an installed cask
let info = installer.info("firefox")?;
println!("Version: {}", info.version);
println!("Path: {}", info.app_path.display());
println!("Artifacts: {:?}", info.artifacts);
```

### Handling Different Artifact Types

```rust
use stout_cask::Artifact;

let cask = index.get_cask("google-chrome")?;

for artifact in &cask.artifacts {
    match artifact {
        Artifact::App(name) => {
            println!("App bundle: {}", name);
        }
        Artifact::Binary(source, target) => {
            println!("Binary: {} → {:?}", source, target);
        }
        Artifact::Pkg(name) => {
            println!("PKG installer: {}", name);
        }
        Artifact::Zap(paths) => {
            println!("Zap paths: {:?}", paths);
        }
    }
}
```

### Quarantine Management

```rust
// Check quarantine status (macOS)
let is_quarantined = installer.is_quarantined("/Applications/Firefox.app")?;

// Remove quarantine attributes
installer.remove_quarantine("/Applications/Firefox.app")?;

// The installer automatically removes quarantine after installation by default
```

### Working with DMG Files

```rust
use stout_cask::DmgHandler;

// Mount a DMG and get the app path inside
let handler = DmgHandler::new()?;
let mount_point = handler.mount("/path/to/download.dmg").await?;

// Find the .app bundle inside
let app = handler.find_app_bundle(&mount_point)?;
println!("Found app: {}", app.display());

// Copy to /Applications
handler.install_app(&app, "/Applications").await?;

// Unmount
handler.unmount(&mount_point).await?;
```

## Platform Support

| Platform | Formats | Notes |
|----------|---------|-------|
| macOS | DMG, ZIP, PKG, TAR | Full support including quarantine |
| Linux | AppImage, Flatpak, DEB, RPM | Best-effort based on distribution |
| Windows | ZIP, MSI, EXE (installer) | Limited support |

## Integration with the Stout Ecosystem

`stout-cask` is the application layer of stout:

- **stout-index** provides cask metadata (URLs, artifacts, versions, checksums)
- **stout-fetch** downloads cask archives
- **stout-state** tracks which casks are installed
- **stout-install** handles the formula side while cask handles applications

You can use `stout-cask` standalone for any project that needs to install macOS or Linux applications programmatically.

## License

MIT License — see the [repository root](../../LICENSE) for details.
