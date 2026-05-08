# stout-state

[![Crates.io](https://img.shields.io/crates/v/stout-state)](https://crates.io/crates/stout-state)
[![Docs.rs](https://docs.rs/stout-state/badge.svg)](https://docs.rs/stout-state)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Lightweight local state management for package installations — tracking installed packages, pins, links, and configuration with a simple file-based API.

**Keywords:** state-management, package-manager, configuration, tracking, json, rust, homebrew, local-state

## Why stout-state?

Every package manager needs to remember what's installed, what version, and where. `stout-state` provides a clean, typed API for managing this local state without a database dependency. It stores everything as readable JSON files, making it easy to inspect, version control, or migrate.

This crate powers the `stout list`, `stout config`, and installation tracking in [stout](https://github.com/neul-labs/stout), but it's designed as a general-purpose state library for any Rust project that needs to persist structured local data.

## Features

- **Installation Tracking** — Track installed formulas, casks, versions, and paths
- **Pin Management** — Pin packages to prevent accidental upgrades
- **Link State** — Track which packages are linked into the prefix
- **Configuration Files** — Typed TOML configuration with defaults and validation
- **Lockfile Support** — Generate and restore reproducible package environments
- **Human-Readable Storage** — Plain JSON files, easy to inspect and version control
- **Atomic Writes** — All updates are atomic (write to temp, then rename)
- **Cross-Platform** — Works on macOS, Linux, and Windows

## Installation

```bash
cargo add stout-state
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-state = "0.2"
```

## Quick Start

```rust
use stout_state::State;

// Load or initialize state in the default location
let state = State::load_default()?;

// Track an installation
state.track_formula("jq", "1.7.1", "/opt/homebrew/Cellar/jq/1.7.1")?;

// Query installed packages
let installed = state.installed_formulas();
for pkg in installed {
    println!("{} {} at {}", pkg.name, pkg.version, pkg.path.display());
}

// Check if a package is pinned
if state.is_pinned("python@3.11")? {
    println!("Python 3.11 is pinned and won't be upgraded");
}
```

## API Overview

### Loading State

```rust
use stout_state::State;

// Default location (~/.stout/state.json)
let state = State::load_default()?;

// Custom state directory
let state = State::load("/var/lib/stout")?;

// In-memory state (useful for testing)
let state = State::in_memory();
```

### Tracking Installations

```rust
// Track a formula installation
state.track_formula(
    "jq",
    "1.7.1",
    "/opt/homebrew/Cellar/jq/1.7.1"
)?;

// Track a cask installation
state.track_cask(
    "firefox",
    "125.0",
    "/Applications/Firefox.app"
)?;

// Update tracking after an upgrade
state.update_version("jq", "1.7.1", "1.7.2")?;

// Untrack (uninstall)
state.untrack_formula("jq")?;
state.untrack_cask("firefox")?;
```

### Querying State

```rust
// List all installed formulas
let formulas = state.installed_formulas();
for f in formulas {
    println!("{} {} — {}", f.name, f.version, f.path.display());
}

// List installed casks
let casks = state.installed_casks();

// Check if a formula is installed
let is_installed = state.has_formula("jq")?;

// Get version of installed package
let version = state.formula_version("jq")?; // Option<String>

// Get installation path
let path = state.formula_path("jq")?; // Option<PathBuf>

// Count installed packages
println!("{} formulas, {} casks installed", 
    state.formula_count(), 
    state.cask_count()
);
```

### Pin Management

```rust
// Pin a package (prevent upgrades)
state.pin("python@3.11")?;

// Unpin
state.unpin("python@3.11")?;

// Check pin status
let is_pinned = state.is_pinned("python@3.11")?;

// List all pinned packages
let pinned = state.pinned_formulas();
for p in pinned {
    println!("Pinned: {} {}", p.name, p.version);
}
```

### Link State

```rust
// Mark a formula as linked
state.mark_linked("jq")?;

// Mark as unlinked
state.mark_unlinked("jq")?;

// Check link status
let is_linked = state.is_linked("jq")?;

// List linked packages
let linked = state.linked_formulas();
```

### Configuration Management

```rust
use stout_state::Config;

// Load configuration
let config = state.config()?;
println!("Cellar: {}", config.install.cellar.display());
println!("Prefix: {}", config.install.prefix.display());

// Update configuration
let mut config = state.config()?;
config.install.parallel_downloads = 8;
config.cache.max_size = "4GB".to_string();
state.save_config(&config)?;

// Access typed config values
let auto_update: bool = state.config_value("index.auto_update")?;
let cellar: PathBuf = state.config_value("install.cellar")?;
```

### Lockfiles

```rust
use stout_state::Lockfile;

// Generate a lockfile from current state
let lockfile = Lockfile::from_state(&state)?;
lockfile.save("Brewfile.lock")?;

// Restore from lockfile
let lockfile = Lockfile::load("Brewfile.lock")?;
for entry in lockfile.entries {
    println!("Restore: {} = {}", entry.name, entry.version);
}

// Check if current state matches lockfile
let is_satisfied = lockfile.is_satisfied_by(&state)?;
if !is_satisfied {
    println!("State doesn't match lockfile — run install to sync");
}
```

### State Inspection

```rust
// Get disk usage summary
let usage = state.disk_usage()?;
println!("Cellar: {}", usage.cellar_size);
println!("Cache: {}", usage.cache_size);
println!("Total: {}", usage.total_size);

// Export state as JSON
let json = state.to_json()?;
println!("{}", json);

// Import state from JSON
state.from_json(&json)?;
```

## Storage Format

State is stored as human-readable JSON files:

```
~/.stout/
├── state.json           — Main state file
├── config.toml          — User configuration
├── receipts/            — Per-package installation receipts
│   ├── jq.json
│   └── node.json
└── lockfiles/           — Generated lockfiles
    └── Brewfile.lock
```

Example `state.json`:

```json
{
  "version": 1,
  "formulas": {
    "jq": {
      "version": "1.7.1",
      "path": "/opt/homebrew/Cellar/jq/1.7.1",
      "installed_at": "2024-05-01T12:00:00Z",
      "pinned": false,
      "linked": true
    }
  },
  "casks": {
    "firefox": {
      "version": "125.0",
      "path": "/Applications/Firefox.app",
      "installed_at": "2024-05-01T12:00:00Z"
    }
  }
}
```

## Integration with the Stout Ecosystem

`stout-state` is the bookkeeping layer of stout:

- **stout-install** updates state after every install/uninstall/upgrade
- **stout-resolve** reads state to skip already-installed dependencies
- **stout-cask** tracks cask installations separately from formulas
- **stout-bundle** generates lockfiles from state and restores from them

You can use `stout-state` standalone for any project that needs simple, persistent, typed local state without pulling in a database.

## License

MIT License — see the [repository root](../../LICENSE) for details.
