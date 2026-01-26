# stout-install

Package installation for stout.

## Overview

This crate handles the installation, removal, and management of formula packages (bottles and source builds).

## Features

- Install bottles (pre-built binaries)
- Build from source with parallel jobs
- Handle tar.gz and tar.zst archives
- Create symlinks to prefix
- Generate INSTALL_RECEIPT.json for Homebrew compatibility
- Dependency installation

## Usage

This crate is primarily used internally by the `stout` CLI through the `stout install` command.

```rust
use stout_install::Installer;

let installer = Installer::new(config)?;
installer.install("jq", &options).await?;
```

## License

MIT License - see the repository root for details.
