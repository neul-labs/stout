# stout-bundle

Brewfile parsing and bundle management for stout.

## Overview

This crate handles Brewfile parsing and bundle operations, allowing users to manage their package installations declaratively through Brewfiles.

## Features

- Parse Brewfile syntax (brew, cask, tap, mas entries)
- Generate Brewfiles from installed packages
- Check if Brewfile requirements are satisfied
- Snapshot and restore functionality

## Usage

This crate is primarily used internally by the `stout` CLI through the `stout bundle` commands.

```rust
use stout_bundle::Brewfile;

let brewfile = Brewfile::parse("Brewfile")?;
let entries = brewfile.entries();
```

## License

MIT License - see the repository root for details.
