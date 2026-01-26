# stout-cask

Cask installation and management for stout.

## Overview

This crate handles the installation, removal, and management of cask applications (macOS .app bundles, Linux AppImages, etc.).

## Features

- Install and uninstall cask applications
- Handle DMG, PKG, and ZIP archives
- Manage application symlinks
- Track installed cask state
- Support for quarantine attributes on macOS

## Usage

This crate is primarily used internally by the `stout` CLI through the `stout cask` commands.

```rust
use stout_cask::CaskInstaller;

let installer = CaskInstaller::new(config)?;
installer.install("firefox").await?;
```

## License

MIT License - see the repository root for details.
