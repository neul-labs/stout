# Stout

**A fast, Rust-based Homebrew-compatible package manager**

Stout is a drop-in replacement for the Homebrew CLI that's 10-100x faster for common operations. It eliminates Ruby entirely and uses pre-computed SQLite indexes with FTS5 full-text search.

---

## Why Stout?

| Operation | brew | stout | Speedup |
|-----------|------|-------|---------|
| `--version` | 500ms | 5ms | **100x** |
| `search json` | 2-5s | <50ms | **40-100x** |
| `info wget` | 1-2s | <100ms | **10-20x** |
| `update` | 10-60s | 1-3s | **10-20x** |

### What Makes Stout Fast

- **No Ruby interpreter** - Native Rust binary with 5ms startup vs 500ms
- **No git operations** - HTTP GET for 1-2MB index vs 700MB+ git repo
- **Pre-computed metadata** - JSON indexed in SQLite, no runtime DSL evaluation
- **SQLite FTS5** - Full-text search indexed database queries
- **Parallel downloads** - Tokio async runtime with concurrent bottle fetching
- **Smart caching** - Hash-based invalidation for formula metadata

---

## Quick Install

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

Or install via package managers:

=== "npm"

    ```bash
    npm install -g stout-pkg
    ```

=== "pip"

    ```bash
    pip install stout-pkg
    ```

=== "gem"

    ```bash
    gem install stout-pkg
    ```

=== "Cargo"

    ```bash
    cargo install stout
    ```

---

## Basic Usage

```bash
# Search for packages
stout search python

# Install a package
stout install jq

# Show package info
stout info wget

# List installed packages
stout list

# Upgrade all packages
stout upgrade
```

---

## Feature Highlights

### Full Homebrew Compatibility

Stout works seamlessly alongside Homebrew:

- Uses the same Cellar structure (`/opt/homebrew/Cellar`)
- Creates compatible `INSTALL_RECEIPT.json` files
- Symlinks to the same prefix (`/opt/homebrew/bin`, etc.)
- Packages installed by either tool are visible to both

### 35+ Commands

Stout supports all common Homebrew operations:

- **Package Management**: install, uninstall, upgrade, update, outdated
- **Discovery**: search, info, list, deps, uses, why
- **Control**: pin, unpin, link, unlink, switch, rollback
- **System**: cleanup, doctor, config, services
- **Casks**: Full macOS/Linux application support
- **Enterprise**: Multi-prefix, bundle, snapshot, audit, mirror

### Security Built-In

- Ed25519 cryptographic signatures on all index updates
- SHA256 checksums for all package downloads
- HTTPS required for all network operations
- Sandboxed extraction with no arbitrary code execution

### Enterprise Ready

- Private index hosting for internal packages
- Multi-prefix isolation for project environments
- Air-gapped deployment with offline mirrors
- Vulnerability scanning with CVE database integration

---

## Next Steps

- [Installation Guide](getting-started.md) - Detailed installation instructions
- [Quick Start](quickstart.md) - Get up and running in minutes
- [Command Reference](commands.md) - Complete command documentation
- [Configuration](configuration.md) - Customize stout behavior
