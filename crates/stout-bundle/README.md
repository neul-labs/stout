# stout-bundle

[![Crates.io](https://img.shields.io/crates/v/stout-bundle)](https://crates.io/crates/stout-bundle)
[![Docs.rs](https://docs.rs/stout-bundle/badge.svg)](https://docs.rs/stout-bundle)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Brewfile parsing, bundle management, and environment snapshots for declarative package management in Rust.

**Keywords:** brewfile, bundle, declarative, package-management, snapshot, ruby-parser, rust, homebrew, infrastructure-as-code

## Why stout-bundle?

Managing a fleet of developer machines or reproducing an environment shouldn't require running commands one by one. `stout-bundle` lets you declare your dependencies in a `Brewfile` and install them with a single command. It also generates Brewfiles from your existing installations and creates named snapshots for rollback.

This crate powers the `stout bundle`, `stout snapshot`, and `stout lock` commands, but it's designed as a general-purpose library for any Rust project that needs to manage sets of packages declaratively.

## Features

- **Brewfile Parsing** — Parse `brew`, `cask`, `tap`, and `mas` entries from Brewfiles
- **Brewfile Generation** — Generate Brewfiles from currently installed packages
- **Bundle Installation** — Install all entries in a Brewfile with dependency resolution
- **Bundle Checking** — Verify if a Brewfile's requirements are satisfied
- **Named Snapshots** — Create and restore named package snapshots
- **Lockfile Support** — Generate lockfiles for reproducible environments
- **Ruby DSL Compatibility** — Supports standard Homebrew Brewfile syntax
- **Custom Directives** — Extensible for custom entry types

## Installation

```bash
cargo add stout-bundle
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-bundle = "0.2"
```

## Quick Start

```rust
use stout_bundle::Brewfile;

// Parse an existing Brewfile
let brewfile = Brewfile::parse("Brewfile")?;

// Print all entries
for entry in brewfile.entries() {
    println!("{:?}", entry);
}

// Generate from installed packages
let brewfile = Brewfile::from_installed()?;
brewfile.save("Brewfile")?;
```

## Brewfile Syntax

Standard Homebrew Brewfile syntax is fully supported:

```ruby
# Taps
tap "homebrew/core"
tap "homebrew/cask"
tap "neul-labs/custom", "https://github.com/neul-labs/homebrew-custom"

# Formulas (CLI tools)
brew "jq"
brew "curl"
brew "git"
brew "postgresql@16", restart_service: true
brew "node@20", link: true

# Casks (applications)
cask "visual-studio-code"
cask "firefox"
cask "docker", args: { appdir: "~/Applications" }

# Mac App Store apps
mas "Xcode", id: 497799835

# Custom paths
brew "custom-tool", path: "/path/to/formula.rb"
```

## API Overview

### Parsing Brewfiles

```rust
use stout_bundle::Brewfile;

// Parse from file
let brewfile = Brewfile::parse("Brewfile")?;

// Parse from string
let brewfile = Brewfile::parse_str(r#"
    tap "homebrew/core"
    brew "jq"
    cask "firefox"
"#)?;

// Parse with custom working directory
let brewfile = Brewfile::parse_with_context("Brewfile", "/path/to/project")?;
```

### Working with Entries

```rust
use stout_bundle::BrewfileEntry;

for entry in brewfile.entries() {
    match entry {
        BrewfileEntry::Tap { name, url } => {
            println!("Tap: {} ({})", name, url.as_deref().unwrap_or("default"));
        }
        BrewfileEntry::Brew { name, options } => {
            println!("Formula: {}", name);
            if options.restart_service {
                println!("  → Will restart service");
            }
        }
        BrewfileEntry::Cask { name, options } => {
            println!("Cask: {}", name);
            if let Some(appdir) = &options.appdir {
                println!("  → App directory: {}", appdir);
            }
        }
        BrewfileEntry::Mas { name, id } => {
            println!("Mac App Store: {} (ID: {})", name, id);
        }
    }
}
```

### Generating Brewfiles

```rust
use stout_bundle::Brewfile;

// Generate from all installed packages
let brewfile = Brewfile::from_installed()?;

// Generate from specific packages
let brewfile = Brewfile::from_packages(
    &["jq", "curl", "git"],           // formulas
    &["firefox", "visual-studio-code"], // casks
    &[],                                // taps (use defaults)
)?;

// Save to file
brewfile.save("Brewfile")?;

// Get as string
let content = brewfile.to_string()?;
```

### Bundle Operations

```rust
use stout_bundle::Bundle;
use stout_index::Index;

let index = Index::open_default()?;
let bundle = Bundle::new(&index);

// Install all entries from a Brewfile
let results = bundle.install("Brewfile").await?;
for result in results {
    match result {
        Ok(name) => println!("Installed: {}", name),
        Err((name, e)) => eprintln!("Failed to install {}: {}", name, e),
    }
}

// Check if Brewfile is satisfied (all packages installed)
let is_satisfied = bundle.check("Brewfile")?;
if !is_satisfied {
    println!("Some packages from the Brewfile are not installed");
}

// List what would be installed (dry run)
let plan = bundle.plan("Brewfile")?;
for entry in plan.to_install {
    println!("Would install: {}", entry);
}
for entry in plan.to_upgrade {
    println!("Would upgrade: {}", entry);
}
```

### Snapshots

```rust
use stout_bundle::Snapshot;

// Create a named snapshot
let snapshot = Snapshot::create("before-upgrade").await?;
println!("Snapshot created: {}", snapshot.id);

// List all snapshots
let snapshots = Snapshot::list()?;
for s in snapshots {
    println!("{} — {} — {}", s.id, s.created_at, s.description);
}

// Restore a snapshot
Snapshot::restore("before-upgrade").await?;

// Delete a snapshot
Snapshot::delete("before-upgrade")?;

// Export snapshot as portable archive
Snapshot::export("before-upgrade", "backup.tar.gz").await?;
Snapshot::import("backup.tar.gz").await?;
```

### Lockfiles

```rust
use stout_bundle::Lockfile;

// Generate lockfile from current state
let lockfile = Lockfile::generate()?;
lockfile.save("Brewfile.lock")?;

// Load and verify
let lockfile = Lockfile::load("Brewfile.lock")?;
if lockfile.is_current()? {
    println!("Lockfile matches current state");
} else {
    println!("State has diverged from lockfile");
}

// Restore exact versions from lockfile
lockfile.restore().await?;
```

### Cleanup

```rust
use stout_bundle::Bundle;

// Remove packages not in the Brewfile
let removed = Bundle::cleanup("Brewfile").await?;
for pkg in removed {
    println!("Removed: {}", pkg);
}

// Preview what would be removed
let to_remove = Bundle::cleanup_plan("Brewfile")?;
for pkg in to_remove {
    println!("Would remove: {}", pkg);
}
```

## Integration with the Stout Ecosystem

`stout-bundle` is the automation layer of stout:

- **stout-index** provides metadata for resolving Brewfile entries
- **stout-resolve** computes installation plans for bundle operations
- **stout-install** executes the installations
- **stout-cask** handles cask entries
- **stout-state** tracks what's installed for generation and checking

You can use `stout-bundle` standalone for any project that needs Brewfile parsing or declarative package management.

## License

MIT License — see the [repository root](../../LICENSE) for details.
