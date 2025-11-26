# brewx

A fast, Rust-based Homebrew-compatible package manager.

[![Build](https://github.com/anthropics/brewx/actions/workflows/ci.yml/badge.svg)](https://github.com/anthropics/brewx/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Why brewx?

brewx is a drop-in replacement for the Homebrew CLI that's **10-100x faster** for common operations:

| Operation | brew | brewx | Speedup |
|-----------|------|-------|---------|
| `--version` | 500ms | 5ms | **100x** |
| `search json` | 2-5s | <50ms | **40-100x** |
| `info wget` | 1-2s | <100ms | **10-20x** |
| `update` | 10-60s | 1-3s | **10-20x** |

The secret? brewx eliminates Ruby entirely. It uses a pre-computed SQLite index with FTS5 full-text search, fetches only what it needs, and downloads bottles in parallel.

## Installation

### Quick Install (Recommended)

Install brewx with a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash
```

This will:
- Detect your OS and architecture
- Download the appropriate binary
- Verify the checksum
- Install to `~/.local/bin` (or `/usr/local/bin` if writable)
- Add to your PATH if needed

#### Installation Options

```bash
# Install to a custom directory
BREWX_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash

# Install a specific version
BREWX_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash

# Skip PATH modification
BREWX_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash
```

### Manual Download

Download pre-built binaries from the [releases page](https://github.com/anthropics/brewx/releases):

| Platform | Architecture | Download |
|----------|-------------|----------|
| macOS | Apple Silicon (M1/M2/M3) | `brewx-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `brewx-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `brewx-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `brewx-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example for macOS ARM
curl -LO https://github.com/anthropics/brewx/releases/latest/download/brewx-aarch64-apple-darwin.tar.gz
tar -xzf brewx-aarch64-apple-darwin.tar.gz
sudo mv brewx /usr/local/bin/
```

### From Source

```bash
# Clone the repository
git clone https://github.com/anthropics/brewx.git
cd brewx

# Build release binary
cargo build --release

# Install to PATH
cp target/release/brewx ~/.local/bin/
# or
sudo cp target/release/brewx /usr/local/bin/
```

### Shell Completions

```bash
# Bash (add to ~/.bashrc)
eval "$(brewx completions bash)"

# Zsh (add to ~/.zshrc)
eval "$(brewx completions zsh)"

# Fish
brewx completions fish > ~/.config/fish/completions/brewx.fish
```

## Quick Start

```bash
# Update the formula index
brewx update

# Search for packages
brewx search json

# Get package info
brewx info jq

# Install a package
brewx install jq

# List installed packages
brewx list

# Check system health
brewx doctor
```

## Commands

| Command | Description |
|---------|-------------|
| `brewx install <pkg>` | Install packages |
| `brewx uninstall <pkg>` | Uninstall packages |
| `brewx search <query>` | Search for packages |
| `brewx info <pkg>` | Show package information |
| `brewx list` | List installed packages |
| `brewx update` | Update the formula index |
| `brewx upgrade` | Upgrade installed packages |
| `brewx doctor` | Check system health |
| `brewx completions <shell>` | Generate shell completions |

## How It Works

brewx uses a hybrid architecture:

1. **SQLite Index** (~3MB): Contains formula metadata for fast queries
2. **Compressed JSON**: Individual formula details fetched on-demand
3. **Homebrew Bottles**: Uses existing Homebrew bottle infrastructure

```
┌─────────────────────────────────────────────────────────────────┐
│                         brewx Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   User ──▶ brewx CLI ──▶ SQLite Index ──▶ Homebrew Bottles      │
│                │              │                   │              │
│                │         FTS5 Search         ghcr.io CDN        │
│                │              │                   │              │
│                └──────────────┴───────────────────┘              │
│                                                                  │
│   Index sync: GitHub raw (brewx-index repo)                      │
│   Bottle download: Homebrew's existing infrastructure            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Compatibility

brewx is designed to work alongside existing Homebrew installations:

- Uses the same Cellar structure (`/opt/homebrew/Cellar`)
- Creates compatible `INSTALL_RECEIPT.json` files
- Symlinks to the same prefix (`/opt/homebrew/bin`, etc.)
- Packages installed by either tool are visible to both

## Configuration

Configuration is stored in `~/.brewx/config.toml`:

```toml
[index]
base_url = "https://raw.githubusercontent.com/anthropics/brewx-index/main"
auto_update = true
update_interval = 1800  # 30 minutes

[install]
cellar = "/opt/homebrew/Cellar"
prefix = "/opt/homebrew"
parallel_downloads = 4

[cache]
max_size = "2GB"
formula_ttl = 86400      # 1 day
download_ttl = 604800    # 7 days
```

## Project Structure

```
brewx/
├── src/                    # CLI application
│   ├── main.rs
│   └── cli/               # Command implementations
├── crates/
│   ├── brewx-index/       # SQLite index management
│   ├── brewx-resolve/     # Dependency resolution
│   ├── brewx-fetch/       # Download management
│   ├── brewx-install/     # Package installation
│   └── brewx-state/       # Local state management
├── scripts/
│   └── sync.py            # Index sync script
├── packaging/             # Package manager distribution
│   ├── homebrew/          # Homebrew formula
│   ├── aur/               # Arch Linux PKGBUILD
│   └── nix/               # Nix flake
├── completions/           # Shell completions
└── docs/                  # Documentation
```

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Usage Guide](docs/USAGE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Technical Specification](docs/SPEC.md)
- [Contributing](docs/CONTRIBUTING.md)
- [Packaging Guide](docs/PACKAGING.md)

## Development

```bash
# Run tests
cargo test --workspace

# Run with verbose logging
RUST_LOG=debug cargo run -- search json

# Build release binary
cargo build --release

# Sync index from Homebrew API (requires Python + uv)
cd scripts && uv run python sync.py --output ../dist
```

## Performance

brewx achieves its speed through several optimizations:

1. **Native binary**: No interpreter startup overhead
2. **SQLite + FTS5**: Instant full-text search
3. **Compressed index**: ~3MB download vs 700MB+ git repo
4. **Parallel downloads**: Concurrent bottle fetching with Tokio
5. **On-demand fetching**: Only download what you need

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Homebrew](https://brew.sh) - The package manager brewx is compatible with
- [uv](https://github.com/astral-sh/uv) - Inspiration for CLI UX design
