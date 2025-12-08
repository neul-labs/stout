# stout Specification

A fast, Rust-based Homebrew-compatible package manager client.

## Overview

stout is a metadata-only package manager that provides a high-performance alternative to the Homebrew CLI. It consumes pre-computed package metadata from a GitHub repository and uses Homebrew's existing bottle infrastructure for actual package artifacts.

**Core Principles:**
- SQLite as the local index for fast queries
- Compressed JSON for full package metadata
- Metadata-only: no artifact hosting, point to upstream bottles
- Modular architecture for maintainability
- Full compatibility with existing Homebrew installations

---

## Why stout is Faster

### Current brew Bottlenecks

| Operation | What brew does | Time |
|-----------|----------------|------|
| **Any command** | Start Ruby interpreter | ~300-500ms |
| **`brew update`** | `git fetch` on homebrew-core (~700MB repo) | 10-60s |
| **`brew search`** | Load all formulas into Ruby, regex match | 2-5s |
| **`brew info`** | Load formula, evaluate Ruby DSL | 1-2s |
| **`brew install`** | Resolve deps in Ruby, sequential downloads | varies |

The fundamental issue: **brew evaluates Ruby on every command**.

```
$ time brew --version
Homebrew 4.2.0
real    0m0.502s   ← 500ms just to print version
```

### How stout Eliminates These

| Operation | What stout does | Time |
|-----------|-----------------|------|
| **Any command** | Start Rust binary | ~5ms |
| **`stout update`** | Download index.db.zst (~1-2MB) | 1-3s |
| **`stout search`** | SQLite FTS5 query | <50ms |
| **`stout info`** | SQLite lookup + fetch 500-byte JSON | <100ms |
| **`stout install`** | Parallel bottle downloads | faster |

### Speedup Sources

```
┌─────────────────────────────────────────────────────────────────┐
│                     BREW (current)                              │
│                                                                 │
│  User ──▶ Ruby ──▶ Load Formulas ──▶ Evaluate DSL ──▶ Result   │
│           500ms      varies            varies                   │
│                                                                 │
│  brew update: git fetch homebrew-core (700MB repo)              │
│  brew search: load ~7000 .rb files, regex each                  │
│  brew install: sequential dep resolution in Ruby                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                     STOUT (proposed)                            │
│                                                                 │
│  User ──▶ Rust ──▶ SQLite Query ──▶ Result                     │
│           5ms        <50ms                                      │
│                                                                 │
│  stout update: download index.db.zst (1-2MB)                    │
│  stout search: SELECT with FTS5 (instant)                       │
│  stout install: parallel async downloads, cached metadata       │
└─────────────────────────────────────────────────────────────────┘
```

### Detailed Breakdown

**1. No Ruby Interpreter**
- brew: Ruby VM startup on every command (~500ms)
- stout: Native Rust binary (~5ms startup)
- **Savings: ~500ms per command**

**2. No Git Operations**
- brew: `git fetch` on 700MB+ homebrew-core repo
- stout: HTTP GET for 1-2MB index file
- **Savings: 10-60s on update**

**3. Pre-computed Metadata**
- brew: Parses Ruby DSL at runtime to get version, deps, etc.
- stout: Already computed, stored as JSON, indexed in SQLite
- **Savings: eliminates Ruby evaluation entirely**

**4. SQLite vs In-Memory Ruby**
- brew: Loads formulas into Ruby objects for search
- stout: FTS5 full-text search on indexed database
- **Savings: 2-5s → <50ms for search**

**5. Parallel Downloads**
- brew: Often sequential operations
- stout: Tokio async runtime, concurrent bottle fetches
- **Savings: N bottles in parallel vs sequential**

**6. Local Caching with Invalidation**
- brew: Re-evaluates formulas each time
- stout: Caches formula JSON, only re-fetches if `json_hash` changed
- **Savings: skip network for unchanged formulas**

### Expected Performance

| Command | brew | stout | Speedup |
|---------|------|-------|---------|
| `--version` | 500ms | 5ms | **100x** |
| `search <query>` | 2-5s | <50ms | **40-100x** |
| `info <pkg>` | 1-2s | <100ms | **10-20x** |
| `update` | 10-60s | 1-3s | **10-20x** |
| `install` (cached) | varies | <10s | **2-5x** |

### What Stays the Same

stout uses the **exact same bottles** as brew:
- Same download URLs (ghcr.io/homebrew/core/...)
- Same checksums
- Same Cellar layout
- Same symlinks

The speedup comes from **how we get there**, not **what we install**.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Data Pipeline                            │
│                                                                 │
│  ┌──────────┐     ┌──────────────┐     ┌───────────────────┐   │
│  │ Homebrew │────▶│  Transform   │────▶│  GitHub Release   │   │
│  │   API    │     │   Scripts    │     │  (index.db.zst)   │   │
│  └──────────┘     └──────────────┘     └───────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Rust CLI                                 │
│                                                                 │
│  ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌────────┐ │
│  │  index   │────▶│ resolve  │────▶│  fetch   │────▶│install │ │
│  └──────────┘     └──────────┘     └──────────┘     └────────┘ │
│       │                                                   │     │
│       ▼                                                   ▼     │
│  ~/.stout/                                    /opt/homebrew/    │
│  index.db                                     Cellar/           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## Data Pipeline

### Source: Homebrew API

Homebrew provides formula metadata via their JSON API:
- `https://formulae.brew.sh/api/formula.json` - All formulas
- `https://formulae.brew.sh/api/cask.json` - All casks

### Transform Scripts

Python scripts that:
1. Fetch latest formula/cask data from Homebrew API
2. Normalize and validate the data
3. Build SQLite index with compressed JSON blobs
4. Push to GitHub repository as a release artifact

#### Script: `scripts/sync.py`

```
Usage: python scripts/sync.py [--full | --incremental]

Options:
  --full         Rebuild entire index from scratch
  --incremental  Only update changed formulas (default)
  --dry-run      Build locally without pushing
  --output DIR   Output directory (default: ./dist)
```

#### Output Artifacts

```
dist/
├── index.db.zst          # Lightweight SQLite index (no blobs)
├── manifest.json         # Version, timestamp, checksums, changed files
└── formulas/
    ├── wget.json.zst
    ├── openssl@3.json.zst
    └── ...               # ~7000 individual formula files
```

#### GitHub Repository Structure

Repository: `github.com/<org>/stout-index`

```
stout-index/
├── index.db.zst          # ~1-2MB (queryable metadata only)
├── manifest.json         # Version info + list of changed formulas
├── formulas/
│   ├── a/
│   │   ├── aom.json.zst
│   │   ├── apr.json.zst
│   │   └── ...
│   ├── b/
│   │   ├── bash.json.zst
│   │   └── ...
│   └── z/
│       └── zstd.json.zst
└── casks/                # Phase 2
    └── ...
```

Files served via GitHub raw URLs or GitHub Pages:
- `https://raw.githubusercontent.com/<org>/stout-index/main/index.db.zst`
- `https://raw.githubusercontent.com/<org>/stout-index/main/formulas/w/wget.json.zst`

#### Automation

GitHub Actions workflows:

| Workflow | Trigger | Frequency | Updates |
|----------|---------|-----------|---------|
| `index-sync.yml` | Scheduled | Every 30 min | Index only |
| `formula-sync.yml` | Homebrew webhook / scheduled | Every 2h | Changed formulas |
| `full-rebuild.yml` | Manual / weekly | Weekly | Everything |

**Why split index and formulas:**
- Index is tiny (~1-2MB) → update very frequently
- Individual formulas fetched on-demand (install, info)
- Git tracks per-formula changes
- CDN caches individual files efficiently

---

## SQLite Index Schema

The index is **lightweight** - it contains only queryable metadata, not full formula data.
Full formula JSON is stored as individual files and fetched on-demand.

### Core Tables

```sql
-- Formula metadata (fast queries, search, listing)
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
    has_bottle INTEGER DEFAULT 1,  -- Quick filter for bottle availability
    json_hash TEXT,                -- SHA256 of formula JSON (for cache invalidation)
    updated_at INTEGER             -- Unix timestamp
);

-- Full-text search
CREATE VIRTUAL TABLE formulas_fts USING fts5(
    name, desc,
    content='formulas',
    content_rowid='rowid'
);

-- Dependencies (for quick dependency queries without fetching JSON)
CREATE TABLE dependencies (
    formula TEXT NOT NULL,
    dep_name TEXT NOT NULL,
    dep_type TEXT NOT NULL,  -- 'runtime', 'build', 'test', 'optional'
    PRIMARY KEY (formula, dep_name, dep_type),
    FOREIGN KEY (formula) REFERENCES formulas(name)
);

-- Bottle availability matrix (quick platform compatibility check)
CREATE TABLE bottles (
    formula TEXT NOT NULL,
    platform TEXT NOT NULL,  -- 'arm64_sonoma', 'x86_64_linux', etc.
    PRIMARY KEY (formula, platform),
    FOREIGN KEY (formula) REFERENCES formulas(name)
);
-- Note: Full bottle URLs/checksums are in the individual JSON files

-- Aliases and old names (for search)
CREATE TABLE aliases (
    alias TEXT PRIMARY KEY,
    formula TEXT NOT NULL,
    FOREIGN KEY (formula) REFERENCES formulas(name)
);

-- Index metadata
CREATE TABLE meta (
    key TEXT PRIMARY KEY,
    value TEXT
);
-- Keys: 'version', 'created_at', 'homebrew_commit', 'formula_count'
```

### What's NOT in the index

The following data lives in individual `formulas/<name>.json.zst` files:
- Full bottle URLs and checksums
- Build instructions
- Caveats text
- Conflicts list
- Full dependency specs (with version constraints)
- Service definitions
- Pour bottle conditions

### Casks (Optional, Phase 2)

```sql
CREATE TABLE casks (
    token TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    name TEXT,              -- Display name
    desc TEXT,
    homepage TEXT,
    url TEXT NOT NULL,
    sha256 TEXT,
    tap TEXT DEFAULT 'homebrew/cask',
    updated_at INTEGER
);

CREATE TABLE cask_json (
    token TEXT PRIMARY KEY,
    data BLOB NOT NULL,
    FOREIGN KEY (token) REFERENCES casks(token)
);
```

---

## Formula Translation

### Source: Homebrew API

We do **not** parse Ruby formulas directly. Homebrew provides a pre-computed JSON API:

```
https://formulae.brew.sh/api/formula.json      # All formulas (~15MB)
https://formulae.brew.sh/api/formula/<name>.json   # Single formula
https://formulae.brew.sh/api/cask.json         # All casks
```

This API is generated by Homebrew's CI when formulas change - Ruby evaluation happens on their side.

### Homebrew API Response (Input)

Example: `https://formulae.brew.sh/api/formula/wget.json`

```json
{
  "name": "wget",
  "full_name": "wget",
  "tap": "homebrew/core",
  "oldname": null,
  "oldnames": [],
  "aliases": [],
  "versioned_formulae": [],
  "desc": "Internet file retriever",
  "license": "GPL-3.0-or-later",
  "homepage": "https://www.gnu.org/software/wget/",
  "versions": {
    "stable": "1.24.5",
    "head": "HEAD",
    "bottle": true
  },
  "urls": {
    "stable": {
      "url": "https://ftp.gnu.org/gnu/wget/wget-1.24.5.tar.gz",
      "tag": null,
      "revision": null,
      "using": null,
      "checksum": "fa2dc35bab5184ecbc46a9ef83def2aaaa3f4c9f3c97d4bd19dcb07d4da637de"
    }
  },
  "revision": 0,
  "version_scheme": 0,
  "bottle": {
    "stable": {
      "rebuild": 0,
      "root_url": "https://ghcr.io/v2/homebrew/core",
      "files": {
        "arm64_sonoma": {
          "cellar": "/opt/homebrew/Cellar",
          "url": "https://ghcr.io/v2/homebrew/core/wget/blobs/sha256:...",
          "sha256": "..."
        },
        "arm64_ventura": { ... },
        "x86_64_linux": { ... }
      }
    }
  },
  "keg_only": false,
  "keg_only_reason": null,
  "options": [],
  "build_dependencies": ["pkg-config"],
  "dependencies": ["libidn2", "openssl@3"],
  "test_dependencies": [],
  "recommended_dependencies": [],
  "optional_dependencies": [],
  "uses_from_macos": [],
  "uses_from_macos_bounds": [],
  "requirements": [],
  "conflicts_with": [],
  "conflicts_with_reasons": [],
  "link_overwrite": [],
  "caveats": null,
  "installed": [],
  "linked_keg": null,
  "pinned": false,
  "outdated": false,
  "deprecated": false,
  "deprecation_date": null,
  "deprecation_reason": null,
  "disabled": false,
  "disable_date": null,
  "disable_reason": null,
  "post_install_defined": false,
  "service": null,
  "tap_git_head": "abc123...",
  "ruby_source_path": "Formula/w/wget.rb",
  "ruby_source_checksum": { "sha256": "..." }
}
```

### Translation Process

```
Homebrew API JSON
       │
       ▼
┌─────────────────────────────────────────────┐
│           Transform Script                  │
│                                             │
│  1. Fetch formula.json (all formulas)       │
│  2. For each formula:                       │
│     a. Extract index fields → SQLite        │
│     b. Normalize full data → stout JSON     │
│     c. Compress with zstd                   │
│  3. Build SQLite index                      │
│  4. Write individual .json.zst files        │
└─────────────────────────────────────────────┘
       │
       ▼
┌──────────────┐    ┌──────────────────────┐
│  index.db    │    │  formulas/<n>.json.zst│
│  (SQLite)    │    │  (individual files)   │
└──────────────┘    └──────────────────────┘
```

### Field Mapping

| Homebrew API Field | Index (SQLite) | Formula JSON |
|--------------------|----------------|--------------|
| `name` | ✓ `formulas.name` | ✓ |
| `versions.stable` | ✓ `formulas.version` | ✓ |
| `revision` | ✓ `formulas.revision` | ✓ |
| `desc` | ✓ `formulas.desc` | ✓ |
| `homepage` | ✓ `formulas.homepage` | ✓ |
| `license` | ✓ `formulas.license` | ✓ |
| `tap` | ✓ `formulas.tap` | ✓ |
| `deprecated` | ✓ `formulas.deprecated` | ✓ |
| `disabled` | ✓ `formulas.disabled` | ✓ |
| `bottle.stable.files` | ✓ `bottles` (platforms only) | ✓ (full URLs) |
| `dependencies` | ✓ `dependencies` | ✓ |
| `build_dependencies` | ✓ `dependencies` | ✓ |
| `optional_dependencies` | ✓ `dependencies` | ✓ |
| `aliases` | ✓ `aliases` | ✓ |
| `urls.stable` | ✗ | ✓ |
| `caveats` | ✗ | ✓ |
| `conflicts_with` | ✗ | ✓ |
| `keg_only` | ✗ | ✓ |
| `service` | ✗ | ✓ |
| `post_install_defined` | ✗ | ✓ |

### stout JSON Schema (Output)

Normalized, flattened structure optimized for the CLI:

```json
{
  "name": "wget",
  "version": "1.24.5",
  "revision": 0,
  "desc": "Internet file retriever",
  "homepage": "https://www.gnu.org/software/wget/",
  "license": "GPL-3.0-or-later",
  "tap": "homebrew/core",

  "urls": {
    "stable": {
      "url": "https://ftp.gnu.org/gnu/wget/wget-1.24.5.tar.gz",
      "sha256": "fa2dc35bab5184ecbc46a9ef83def2aaaa3f4c9f3c97d4bd19dcb07d4da637de"
    },
    "head": "https://git.savannah.gnu.org/git/wget.git"
  },

  "bottles": {
    "arm64_sonoma": {
      "url": "https://ghcr.io/v2/homebrew/core/wget/blobs/sha256:...",
      "sha256": "...",
      "cellar": "/opt/homebrew/Cellar"
    },
    "arm64_ventura": { ... },
    "x86_64_linux": { ... }
  },

  "dependencies": {
    "runtime": ["libidn2", "openssl@3"],
    "build": ["pkg-config"],
    "test": [],
    "optional": [],
    "recommended": []
  },

  "aliases": [],
  "conflicts_with": [],
  "caveats": null,

  "flags": {
    "keg_only": false,
    "deprecated": false,
    "disabled": false,
    "has_post_install": false
  },

  "service": null,

  "meta": {
    "ruby_source_path": "Formula/w/wget.rb",
    "tap_git_head": "abc123..."
  }
}
```

### Sync Script Pseudocode

```python
def sync():
    # 1. Fetch all formulas from Homebrew API
    formulas = fetch_json("https://formulae.brew.sh/api/formula.json")

    # 2. Initialize SQLite database
    db = create_database("index.db")

    for formula in formulas:
        # 3. Extract fields for index (fast queries)
        db.insert_formula(
            name=formula["name"],
            version=formula["versions"]["stable"],
            revision=formula["revision"],
            desc=formula["desc"],
            homepage=formula["homepage"],
            license=formula["license"],
            tap=formula["tap"],
            deprecated=formula["deprecated"],
            disabled=formula["disabled"],
            has_bottle=bool(formula.get("bottle")),
        )

        # 4. Insert dependencies
        for dep in formula.get("dependencies", []):
            db.insert_dependency(formula["name"], dep, "runtime")
        for dep in formula.get("build_dependencies", []):
            db.insert_dependency(formula["name"], dep, "build")
        # ... etc

        # 5. Insert bottle platforms
        if bottles := formula.get("bottle", {}).get("stable", {}).get("files"):
            for platform in bottles.keys():
                db.insert_bottle(formula["name"], platform)

        # 6. Transform to stout JSON format
        stout_json = transform_formula(formula)

        # 7. Compress and write individual file
        compressed = zstd.compress(json.dumps(stout_json))
        write_file(f"formulas/{formula['name'][0]}/{formula['name']}.json.zst", compressed)

        # 8. Store hash in index for cache invalidation
        json_hash = sha256(compressed)
        db.update_formula_hash(formula["name"], json_hash)

    # 9. Build FTS index
    db.rebuild_fts()

    # 10. Write manifest
    write_manifest(db.formula_count(), db.version())
```

### Compression Details

- **Algorithm**: zstd (level 19 for max compression)
- **Typical ratio**: 5-10x compression
- **Per-formula**: ~200-500 bytes compressed (from 2-5KB JSON)
- **Total formulas dir**: ~3-5MB for all ~7000 formulas

---

## Rust CLI

### Crate Structure

```
stout/
├── Cargo.toml
├── crates/
│   ├── stout-index/       # SQLite index management
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── db.rs      # Database operations
│   │   │   ├── schema.rs  # Table definitions
│   │   │   ├── sync.rs    # Index download/update
│   │   │   └── query.rs   # Search, lookup
│   │   └── Cargo.toml
│   │
│   ├── stout-resolve/     # Dependency resolution
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── graph.rs   # Dependency graph
│   │   │   ├── solver.rs  # SAT/pubgrub solver
│   │   │   └── plan.rs    # Installation plan
│   │   └── Cargo.toml
│   │
│   ├── stout-fetch/       # Download management
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── client.rs  # HTTP client (reqwest)
│   │   │   ├── progress.rs
│   │   │   ├── verify.rs  # Checksum verification
│   │   │   └── cache.rs   # Download cache
│   │   └── Cargo.toml
│   │
│   ├── stout-install/     # Package installation
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── extract.rs # Tar/bottle extraction
│   │   │   ├── link.rs    # Symlink to bin/
│   │   │   ├── receipt.rs # INSTALL_RECEIPT.json
│   │   │   └── hooks.rs   # Post-install scripts
│   │   └── Cargo.toml
│   │
│   └── stout-state/       # Local state management
│       ├── src/
│       │   ├── lib.rs
│       │   ├── config.rs  # User configuration
│       │   ├── installed.rs # Installed packages DB
│       │   └── lock.rs    # Lockfile support
│       └── Cargo.toml
│
└── src/
    ├── main.rs
    └── cli/
        ├── mod.rs
        ├── install.rs
        ├── uninstall.rs
        ├── search.rs
        ├── info.rs
        ├── update.rs
        ├── list.rs
        └── upgrade.rs
```

### Key Dependencies

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
rusqlite = { version = "0.31", features = ["bundled"] }
zstd = "0.13"
reqwest = { version = "0.12", features = ["stream", "rustls-tls"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
indicatif = "0.17"          # Progress bars
sha2 = "0.10"               # Checksum verification
tar = "0.4"
flate2 = "1"                # gzip for bottles
dirs = "5"                  # XDG paths
```

### CLI Commands

```
stout <command> [options]

Commands:
  install <formula>...    Install packages
  uninstall <formula>...  Remove packages
  upgrade [formula]...    Upgrade packages (all if none specified)
  search <query>          Search formulas
  info <formula>          Show package details
  list                    List installed packages
  update                  Update formula index
  doctor                  Check system health
  config                  Manage configuration

Options:
  -v, --verbose           Verbose output
  -q, --quiet             Suppress output
  --dry-run               Show what would be done
  --offline               Use cached index only
```

### CLI UX Design

Inspired by `uv` - fast, beautiful, informative.

#### Color Palette

```
Primary:    Cyan    (#06b6d4)  - Actions, commands
Success:    Green   (#22c55e)  - Completed, installed
Warning:    Yellow  (#eab308)  - Warnings, deprecations
Error:      Red     (#ef4444)  - Errors, failures
Muted:      Gray    (#6b7280)  - Secondary info, paths
Accent:     Blue    (#3b82f6)  - Links, versions
```

#### Output Examples

**Install:**
```
$ stout install wget ripgrep fd

Resolving dependencies...
  ✓ wget 1.24.5
  ✓ ripgrep 14.1.0
  ✓ fd 9.0.0
  + openssl@3 3.2.1 (dependency)
  + pcre2 10.42 (dependency)

Downloading 5 packages...
  ████████████████████████████████████████ 5/5 (12.4 MB)

Installing...
  ✓ openssl@3 3.2.1
  ✓ pcre2 10.42
  ✓ wget 1.24.5
  ✓ ripgrep 14.1.0
  ✓ fd 9.0.0

Installed 5 packages in 3.2s
```

**Search:**
```
$ stout search json

Found 47 formulas:

  jq 1.7.1                Command-line JSON processor
  fx 31.0.0               Terminal JSON viewer
  gojq 0.12.14            Pure Go implementation of jq
  jless 0.9.0             Command-line JSON viewer
  jsonlint 1.6.3          JSON parser and validator
  ...

Use 'stout info <formula>' for details
```

**Update:**
```
$ stout update

Fetching index...
  ████████████████████████████████████████ 1.8 MB

Updated to 2024.01.15.042 (7,012 formulas)
  + 3 new: foo, bar, baz
  ↑ 12 updated

Last sync: 2 minutes ago
```

**Info:**
```
$ stout info wget

wget 1.24.5
Internet file retriever

Homepage:  https://www.gnu.org/software/wget/
License:   GPL-3.0-or-later
Tap:       homebrew/core

Dependencies:
  ├── openssl@3 (runtime)
  ├── libidn2 (runtime)
  └── pkg-config (build)

Bottles:
  ✓ arm64_sonoma    ✓ arm64_ventura    ✓ x86_64_linux
  ✓ sonoma          ✓ ventura

Installed: No
```

**Upgrade:**
```
$ stout upgrade

Checking for updates...

2 packages can be upgraded:

  Package     Current    Latest
  ─────────────────────────────
  wget        1.24.4  →  1.24.5
  openssl@3   3.2.0   →  3.2.1

Upgrade all? [Y/n] y

Downloading...
  ████████████████████████████████████████ 2/2 (8.1 MB)

Installing...
  ✓ openssl@3 3.2.0 → 3.2.1
  ✓ wget 1.24.4 → 1.24.5

Upgraded 2 packages in 2.1s
```

**List:**
```
$ stout list

Installed packages (23):

  Name          Version     Size      Installed
  ────────────────────────────────────────────────
  wget          1.24.5      4.2 MB    2 days ago
  ripgrep       14.1.0      6.1 MB    1 week ago
  fd            9.0.0       3.8 MB    1 week ago
  jq            1.7.1       1.2 MB    2 weeks ago
  ...

Total: 23 packages, 142 MB
```

**Errors:**
```
$ stout install nonexistent

error: formula 'nonexistent' not found

Did you mean?
  • node
  • neotest
  • nextest

Run 'stout search <query>' to find packages
```

#### Progress Indicators

```rust
// Use indicatif with custom styles
use indicatif::{ProgressBar, ProgressStyle};

// Download progress
let style = ProgressStyle::default_bar()
    .template("{spinner:.cyan} {msg}\n  {bar:40.cyan/dim} {pos}/{len} ({bytes})")
    .progress_chars("━━╸━");

// Spinner for resolving
let style = ProgressStyle::default_spinner()
    .template("{spinner:.cyan} {msg}")
    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏");
```

#### Design Principles

1. **Instant feedback** - Show spinner immediately, never hang silently
2. **Progressive disclosure** - Summary by default, `--verbose` for details
3. **Scannable output** - Aligned columns, clear hierarchy
4. **Color with purpose** - Green=success, red=error, cyan=action
5. **Helpful errors** - Suggest fixes, show similar matches
6. **Timing info** - Show how fast we are ("in 3.2s")
7. **No clutter** - No unnecessary blank lines or decorations

#### Key Dependencies for UX

```toml
indicatif = "0.17"      # Progress bars and spinners
console = "0.15"        # Terminal colors and styling
dialoguer = "0.11"      # Interactive prompts (Y/n)
unicode-width = "0.1"   # Proper column alignment
humansize = "2"         # "4.2 MB" formatting
humantime = "2"         # "2 days ago" formatting
```

---

## Local State

### Directory Structure

```
~/.stout/
├── config.toml           # User configuration
├── index.db              # SQLite index (decompressed, ~2-5MB)
├── manifest.json         # Current index version info
├── cache/
│   ├── formulas/         # Cached formula JSONs (decompressed)
│   │   ├── wget.json
│   │   ├── openssl@3.json
│   │   └── ...
│   └── downloads/        # Downloaded bottles (cleared periodically)
└── state/
    └── installed.toml    # Local installation tracking
```

### config.toml

```toml
[index]
# Base URL for stout-index repository (raw GitHub content)
base_url = "https://raw.githubusercontent.com/<org>/stout-index/main"
auto_update = true
update_interval = 1800  # seconds (30 min for index)

[install]
cellar = "/opt/homebrew/Cellar"
prefix = "/opt/homebrew"
parallel_downloads = 4

[cache]
max_size = "2GB"
formula_ttl = 86400     # 1 day - formula JSONs
download_ttl = 604800   # 7 days - bottle downloads
```

### installed.toml

```toml
# Tracks stout-managed installations
# Coexists with Homebrew's INSTALL_RECEIPT.json

[wget]
version = "1.24.5"
revision = 0
installed_at = 2024-01-15T10:30:00Z
installed_by = "stout"
requested = true  # Explicitly installed vs dependency

[openssl@3]
version = "3.2.1"
revision = 0
installed_at = 2024-01-15T10:29:55Z
installed_by = "stout"
requested = false  # Installed as dependency of wget
```

---

## Homebrew Compatibility

### Cellar Layout

stout uses the same Cellar structure as Homebrew:

```
/opt/homebrew/
├── Cellar/
│   └── wget/
│       └── 1.24.5/
│           ├── bin/
│           │   └── wget
│           ├── share/
│           ├── INSTALL_RECEIPT.json
│           └── .brew/
│               └── wget.rb  (optional, for `brew` compat)
├── bin/                     # Symlinks
│   └── wget -> ../Cellar/wget/1.24.5/bin/wget
└── opt/
    └── wget -> ../Cellar/wget/1.24.5
```

### INSTALL_RECEIPT.json

stout writes compatible receipts so `brew` can see stout-installed packages:

```json
{
  "homebrew_version": "4.x.x",
  "installed_as_dependency": false,
  "installed_on_request": true,
  "install_time": 1705312200,
  "source": {
    "tap": "homebrew/core"
  },
  "runtime_dependencies": [
    {"full_name": "openssl@3", "version": "3.2.1"}
  ]
}
```

### Side-by-Side Operation

- stout reads existing Homebrew installations
- `brew` can see stout-installed packages
- Users can mix `brew` and `stout` commands
- No conflicts: same paths, same receipts

---

## Sync Protocol

### Index Update (`stout update`)

```
1. GET manifest.json (ETag/If-Modified-Since for caching)
2. Compare manifest.index_version vs local version
3. If different:
   a. GET index.db.zst
   b. Verify sha256
   c. Decompress to ~/.stout/index.db
4. Note: Individual formula JSONs are NOT fetched here
```

### On-Demand Formula Fetch (`stout install/info`)

```
1. Check local cache: ~/.stout/cache/formulas/<name>.json
2. Compare cached json_hash vs index.formulas.json_hash
3. If missing or stale:
   a. GET formulas/<first-letter>/<name>.json.zst
   b. Decompress and cache locally
   c. Store json_hash for future validation
4. Use cached formula data
```

### Local Formula Cache

```
~/.stout/cache/formulas/
├── wget.json              # Decompressed, ready to use
├── openssl@3.json
└── ...
```

Cache invalidation via `json_hash` in index - no need to re-download if hash matches.

### Manifest Format

```json
{
  "version": "2024.01.15.042",
  "index_version": "2024.01.15.042",
  "index_sha256": "abc123...",
  "index_size": 1847293,
  "formula_count": 7012,
  "created_at": "2024-01-15T10:30:00Z",
  "homebrew_commit": "abc123def456"
}
```

### Offline Mode

When `--offline` or network unavailable:
- Use existing index.db
- Use cached formula JSONs
- Warn if index is stale (>24h)
- Fail gracefully if formula JSON not cached

---

## Security

### Checksums

- All downloads verified against sha256 in metadata
- Index itself verified against manifest checksum
- Failed verification = abort install

### Future Considerations

- Signed manifests (GPG or minisign)
- Content-addressable cache
- Pinned certificates for GitHub/GHCR

---

## Performance Targets

| Operation | Target |
|-----------|--------|
| `stout search <query>` | <50ms |
| `stout info <formula>` | <100ms |
| `stout update` (incremental) | <5s |
| `stout install` (cached bottle) | <10s |
| Index size (compressed) | <10MB |

---

## Phases

### Phase 1: Core CLI
- [ ] SQLite index crate
- [ ] Sync from GitHub releases
- [ ] search, info, list commands
- [ ] install (bottles only)
- [ ] uninstall, upgrade

### Phase 2: Full Feature Parity
- [ ] Cask support
- [ ] Build from source fallback
- [ ] Tap support (custom indexes)
- [ ] Lockfile support

### Phase 3: Enhancements
- [ ] Parallel installs
- [ ] Signed indexes
- [ ] Delta sync optimization
- [ ] Shell completions
