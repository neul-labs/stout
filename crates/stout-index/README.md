# stout-index

[![Crates.io](https://img.shields.io/crates/v/stout-index)](https://crates.io/crates/stout-index)
[![Docs.rs](https://docs.rs/stout-index/badge.svg)](https://docs.rs/stout-index)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

A high-performance SQLite-backed index for Homebrew formula and cask metadata, with FTS5 full-text search and Ed25519 signature verification.

**Keywords:** homebrew, package-manager, sqlite, fts5, full-text-search, index, rust, formula, cask

## Why stout-index?

If you're building a tool that needs to query Homebrew package metadata, you have two options: clone the 700MB+ Homebrew git repository and parse Ruby files, or use a pre-computed index. `stout-index` provides the latter — a ~3MB SQLite database with sub-50ms full-text search over 8,000+ formulas and 4,000+ casks.

This crate powers the search, info, and dependency resolution in [stout](https://github.com/neul-labs/stout), but it's designed as a reusable library for any Rust project that needs fast access to Homebrew metadata.

## Features

- **FTS5 Full-Text Search** — Search package names, descriptions, and tags with stemming and prefix matching
- **Tiny Footprint** — ~3MB compressed index vs 700MB+ git repository
- **Cryptographic Integrity** — Ed25519 signature verification on every index update
- **Zero-Copy Queries** — Read-optimized schema with covering indexes for common lookups
- **Async Update Support** — Download and verify index updates without blocking
- **Formula & Cask Metadata** — Versions, dependencies, URLs, checksums, caveats, and more
- **Dependency Graph Queries** — Direct and transitive dependency lookups
- **Version History** — Track formula changes across index versions

## Installation

```bash
cargo add stout-index
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-index = "0.2"
```

## Quick Start

```rust
use stout_index::Index;

// Open (or create) the local index
let index = Index::open("~/.stout/index.db")?;

// Full-text search across formulas and casks
let results = index.search("json parser")?;
for formula in results {
    println!("{} — {}", formula.name, formula.description);
}

// Get detailed formula metadata
let jq = index.get_formula("jq")?;
println!("Latest version: {}", jq.versions.stable);
println!("Homepage: {}", jq.homepage);

// Query dependencies
let deps = index.get_dependencies("wget")?;
println!("wget depends on: {:?}", deps);
```

## API Overview

### Opening the Index

```rust
use stout_index::Index;

// Default location
let index = Index::open_default()?;

// Custom path
let index = Index::open("/var/lib/stout/index.db")?;

// In-memory (useful for testing)
let index = Index::open_in_memory()?;
```

### Full-Text Search

```rust
// Search formulas by name, description, and tags
let formulas = index.search_formulas("web server")?;

// Search casks (applications)
let casks = index.search_casks("code editor")?;

// Combined search
let all = index.search("database")?;
```

### Formula Metadata

```rust
let formula = index.get_formula("postgresql@16")?;

println!("Name: {}", formula.name);
println!("Description: {}", formula.description);
println!("Stable version: {}", formula.versions.stable);
println!("Homepage: {}", formula.homepage);
println!("License: {}", formula.license);
println!("Bottle available: {}", formula.bottle.is_some());

// Dependencies by type
println!("Build deps: {:?}", formula.build_dependencies);
println!("Required deps: {:?}", formula.dependencies);
println!("Optional deps: {:?}", formula.optional_dependencies);

// Checksum for integrity verification
println!("Source SHA256: {}", formula.source_checksum);
```

### Cask Metadata

```rust
let cask = index.get_cask("visual-studio-code")?;

println!("Name: {}", cask.name);
println!("Version: {}", cask.version);
println!("Download URL: {}", cask.url);
println!("SHA256: {}", cask.sha256);
println!("Appcast: {:?}", cask.appcast);

// Artifact types (app, pkg, binary, etc.)
for artifact in cask.artifacts {
    println!("Artifact: {:?}", artifact);
}
```

### Index Updates

```rust
use stout_index::{Index, UpdateOptions};

// Check if an update is available
if index.update_available()? {
    // Download latest index with signature verification
    let options = UpdateOptions {
        base_url: "https://raw.githubusercontent.com/neul-labs/stout-index/main".to_string(),
        verify_signatures: true,
        public_key: Some(ed25519_public_key),
    };
    
    index.update(options).await?;
}

// Get index version info
let meta = index.metadata()?;
println!("Index version: {}", meta.version);
println!("Formula count: {}", meta.formula_count);
println!("Cask count: {}", meta.cask_count);
println!("Last updated: {}", meta.updated_at);
```

### Advanced Queries

```rust
// List all formulas in a category
let dev_tools = index.formulas_by_tag("development")?;

// Check if a formula exists
let exists = index.has_formula("node")?;

// Get all versions of a formula
let versions = index.formula_versions("python")?; // python, python@3.11, python@3.12, ...

// Find formulas that depend on a given package
let dependents = index.reverse_dependencies("openssl")?;

// Search by license
let mit_packages = index.formulas_by_license("MIT")?;
```

## Performance

Typical query latencies on a modern SSD:

| Query Type | Latency | Notes |
|------------|---------|-------|
| `get_formula` | <1ms | Primary key lookup |
| `search` (single term) | 5-15ms | FTS5 indexed |
| `search` (multi-term) | 10-30ms | FTS5 with ranking |
| `get_dependencies` | <5ms | Indexed join |
| `reverse_dependencies` | 10-50ms | Depends on result set |
| Index update | 1-3s | Download + verify + replace |

The SQLite database is typically 15-25MB uncompressed, 2-4MB compressed for transfer.

## Schema Design

The index uses a normalized schema optimized for read-heavy workloads:

```
formulas          — Core formula metadata
  └─ formula_dependencies — Dependency links
  └─ formula_versions     — Historical versions
casks             — Core cask metadata
  └─ cask_artifacts         — Installable artifacts
fts_formulas      — FTS5 virtual table for formula search
fts_casks         — FTS5 virtual table for cask search
index_metadata    — Version, checksum, signature
```

## Security

Index updates are cryptographically signed with Ed25519. The crate verifies signatures before applying updates, preventing tampering with package metadata.

```rust
use stout_index::SignatureVerifier;

let verifier = SignatureVerifier::new(&public_key_bytes)?;
let is_valid = verifier.verify(&index_data, &signature)?;
```

## Integration with the Stout Ecosystem

`stout-index` is the foundation of the stout architecture:

- **stout-resolve** consumes the index to build dependency graphs
- **stout-fetch** uses index URLs to locate bottles and source tarballs
- **stout-install** queries the index for installation metadata
- **stout-cask** reads cask artifacts and version info from the index
- **stout-audit** cross-references installed packages against index metadata

You can use `stout-index` standalone if you only need metadata queries, or combine it with other stout crates for a complete package management solution.

## License

MIT License — see the [repository root](../../LICENSE) for details.
