# stout-index

SQLite index management for stout.

## Overview

This crate manages the SQLite database that contains formula and cask metadata. It provides fast full-text search using FTS5 and handles index updates with Ed25519 signature verification.

## Features

- SQLite database with FTS5 full-text search
- Compressed index download and updates
- Ed25519 signature verification
- Formula and cask metadata queries
- Version and dependency information

## Usage

This crate is the core data layer used by all other stout crates.

```rust
use stout_index::Index;

let index = Index::open()?;
let results = index.search("json")?;
let formula = index.get_formula("jq")?;
```

## License

MIT License - see the repository root for details.
