# Architecture

This document describes the internal architecture of brewx.

## Overview

brewx is structured as a Rust workspace with multiple crates, each handling a specific concern:

```
brewx/
├── src/                    # Main CLI binary
├── crates/
│   ├── brewx-index/       # SQLite index management
│   ├── brewx-resolve/     # Dependency resolution
│   ├── brewx-fetch/       # Download management
│   ├── brewx-install/     # Package installation
│   └── brewx-state/       # Local state management
└── scripts/
    └── sync.py            # Index sync script
```

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              User                                        │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           brewx CLI (src/)                               │
│  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐           │
│  │ install │ │ search  │ │  info   │ │  list   │ │ doctor  │  ...      │
│  └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘           │
└───────┼──────────┼──────────┼──────────┼──────────┼─────────────────────┘
        │          │          │          │          │
        ▼          ▼          ▼          ▼          ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         Crate Layer                                      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                   │
│  │ brewx-index  │  │brewx-resolve │  │ brewx-fetch  │                   │
│  │  (SQLite)    │  │  (Dep Graph) │  │ (Downloads)  │                   │
│  └──────────────┘  └──────────────┘  └──────────────┘                   │
│  ┌──────────────┐  ┌──────────────┐                                     │
│  │brewx-install │  │ brewx-state  │                                     │
│  │  (Extraction)│  │  (Config)    │                                     │
│  └──────────────┘  └──────────────┘                                     │
└─────────────────────────────────────────────────────────────────────────┘
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐  ┌───────────────┐  ┌───────────────────────────────────┐
│  ~/.brewx/    │  │  Homebrew     │  │        Network                    │
│  index.db     │  │  Cellar       │  │  ┌─────────────────────────────┐  │
│  config.toml  │  │  /opt/homebrew│  │  │ brewx-index (GitHub raw)    │  │
│  state/       │  │               │  │  │ Homebrew bottles (ghcr.io)  │  │
└───────────────┘  └───────────────┘  │  └─────────────────────────────┘  │
                                      └───────────────────────────────────┘
```

## Crate Details

### brewx-index

**Purpose**: SQLite database management and formula queries.

**Key Components**:
- `Database`: SQLite connection wrapper with transaction support
- `Query`: High-level query interface
- `IndexSync`: Remote index synchronization
- `Formula`/`FormulaInfo`: Formula data structures

**Schema**:
```sql
-- Main formula table
CREATE TABLE formulas (
    name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    revision INTEGER DEFAULT 0,
    desc TEXT,
    homepage TEXT,
    license TEXT,
    tap TEXT DEFAULT 'homebrew/core',
    deprecated INTEGER DEFAULT 0,
    disabled INTEGER DEFAULT 0,
    has_bottle INTEGER DEFAULT 1,
    json_hash TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE formulas_fts USING fts5(name, desc);

-- Dependencies
CREATE TABLE dependencies (
    formula TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    dep_type TEXT NOT NULL  -- runtime, build, test, optional, recommended
);

-- Bottle availability
CREATE TABLE bottles (
    formula TEXT NOT NULL,
    platform TEXT NOT NULL  -- arm64_sonoma, x86_64_linux, etc.
);
```

### brewx-resolve

**Purpose**: Dependency graph construction and resolution.

**Key Components**:
- `DependencyGraph`: DAG representation of package dependencies
- `InstallPlan`: Ordered list of packages to install
- Topological sort with cycle detection

**Algorithm**:
```
1. Start with requested packages
2. BFS to collect all dependencies
3. Build directed graph (package -> dependencies)
4. Topological sort (Kahn's algorithm)
5. Detect cycles (if sort incomplete)
6. Return install order (deps first)
```

### brewx-fetch

**Purpose**: Download management with caching and verification.

**Key Components**:
- `DownloadClient`: HTTP client with connection pooling
- `DownloadCache`: Local cache for downloaded bottles
- `ProgressReporter`: Multi-progress bar display
- SHA256 verification

**Features**:
- Parallel downloads with semaphore-based concurrency control
- Automatic retry on failure
- Cache-first fetching
- Progress reporting for each download

### brewx-install

**Purpose**: Package installation and symlink management.

**Key Components**:
- `extract_bottle()`: Extracts .tar.gz bottles to Cellar
- `link_package()`: Creates symlinks to prefix
- `write_receipt()`: Creates INSTALL_RECEIPT.json
- `remove_package()`: Cleanup on uninstall

**Directory Structure**:
```
/opt/homebrew/
├── Cellar/
│   └── jq/
│       └── 1.7.1/
│           ├── bin/jq
│           ├── lib/
│           ├── share/
│           └── INSTALL_RECEIPT.json
├── bin/
│   └── jq -> ../Cellar/jq/1.7.1/bin/jq
└── opt/
    └── jq -> ../Cellar/jq/1.7.1
```

### brewx-state

**Purpose**: Configuration and local state management.

**Key Components**:
- `Config`: User configuration (config.toml)
- `InstalledPackages`: Tracks installed packages (installed.toml)
- `Paths`: Standard directory locations

**Files**:
```
~/.brewx/
├── config.toml         # User configuration
├── index.db            # Formula index (SQLite)
├── manifest.json       # Index metadata
├── state/
│   └── installed.toml  # Installed packages tracking
└── cache/
    ├── formulas/       # Cached formula JSON
    └── downloads/      # Cached bottles
```

## Data Flow

### Search Operation

```
User: brewx search json
         │
         ▼
    ┌─────────┐
    │ CLI     │ Parse arguments
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ Index   │ Open ~/.brewx/index.db
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ SQLite  │ FTS5 MATCH query
    └────┬────┘
         │
         ▼
    ┌─────────┐
    │ Output  │ Format and display results
    └─────────┘
```

### Install Operation

```
User: brewx install jq
         │
         ▼
    ┌─────────────┐
    │ CLI         │ Parse arguments
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │ Index       │ Look up formula, get deps
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │ Resolve     │ Build dependency graph
    │             │ Topological sort
    │             │ Create install plan
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │ Fetch       │ Download bottles (parallel)
    │             │ Verify checksums
    │             │ Cache locally
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │ Install     │ Extract to Cellar
    │             │ Create symlinks
    │             │ Write receipt
    └──────┬──────┘
           │
           ▼
    ┌─────────────┐
    │ State       │ Update installed.toml
    └─────────────┘
```

## Index Sync Architecture

The index is maintained separately and synced to clients:

```
┌───────────────────────────────────────────────────────────────┐
│                    Index Generation (sync.py)                  │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│  Homebrew API ──▶ Transform ──▶ SQLite + JSON ──▶ GitHub      │
│  (formula.json)    (Python)      (compressed)     (raw files) │
│                                                                │
└───────────────────────────────────────────────────────────────┘

┌───────────────────────────────────────────────────────────────┐
│                    Index Consumption (brewx)                   │
├───────────────────────────────────────────────────────────────┤
│                                                                │
│  GitHub raw ──▶ Download ──▶ Decompress ──▶ Local SQLite      │
│  (index.db.zst)  (HTTP GET)   (zstd)        (~/.brewx/)       │
│                                                                │
└───────────────────────────────────────────────────────────────┘
```

## Concurrency Model

brewx uses Tokio for async operations:

```rust
// Parallel bottle downloads with semaphore
pub async fn download_bottles(
    &self,
    bottles: Vec<BottleSpec>,
    progress: Arc<ProgressReporter>,
) -> Result<Vec<PathBuf>> {
    let futures: Vec<_> = bottles
        .into_iter()
        .map(|spec| {
            let client = self.clone();
            let progress = Arc::clone(&progress);
            async move {
                // Acquire semaphore permit (limits concurrency)
                let _permit = client.semaphore.acquire().await?;
                client.download_bottle(spec, progress).await
            }
        })
        .collect();

    join_all(futures).await
}
```

## Error Handling

Each crate defines its own error type using `thiserror`:

```rust
#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Formula not found: {0}")]
    FormulaNotFound(String),

    #[error("Dependency cycle detected: {0}")]
    CycleDetected(String),
    // ...
}
```

Errors propagate up through the crate hierarchy and are handled at the CLI level with user-friendly messages.

## Testing Strategy

- **Unit tests**: Each crate has a `tests.rs` module
- **Integration tests**: Test full workflows with temp directories
- **122 total tests** covering all crates

```bash
cargo test --workspace
```
