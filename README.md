# stout

A fast, Rust-based Homebrew-compatible package manager.

[![Build](https://github.com/neul-labs/stout/actions/workflows/ci.yml/badge.svg)](https://github.com/neul-labs/stout/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

## Why stout?

stout is a drop-in replacement for the Homebrew CLI that's **10-100x faster** for common operations:

| Operation | brew | stout | Speedup |
|-----------|------|-------|---------|
| `--version` | 500ms | 5ms | **100x** |
| `search json` | 2-5s | <50ms | **40-100x** |
| `info wget` | 1-2s | <100ms | **10-20x** |
| `update` | 10-60s | 1-3s | **10-20x** |

The secret? stout eliminates Ruby entirely. It uses a pre-computed SQLite index with FTS5 full-text search, fetches only what it needs, and downloads bottles in parallel.

## Installation

### Quick Install (Recommended)

Install stout with a single command:

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
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
STOUT_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash

# Install a specific version
STOUT_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash

# Skip PATH modification
STOUT_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

### Manual Download

Download pre-built binaries from the [releases page](https://github.com/neul-labs/stout/releases):

| Platform | Architecture | Download |
|----------|-------------|----------|
| macOS | Apple Silicon (M1/M2/M3) | `stout-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `stout-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `stout-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `stout-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example for macOS ARM
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout-aarch64-apple-darwin.tar.gz
tar -xzf stout-aarch64-apple-darwin.tar.gz
sudo mv stout /usr/local/bin/
```

### From Source

```bash
# Clone the repository
git clone https://github.com/neul-labs/stout.git
cd stout

# Build release binary
cargo build --release

# Install to PATH
cp target/release/stout ~/.local/bin/
# or
sudo cp target/release/stout /usr/local/bin/
```

### Shell Completions

```bash
# Bash (add to ~/.bashrc)
eval "$(stout completions bash)"

# Zsh (add to ~/.zshrc)
eval "$(stout completions zsh)"

# Fish
stout completions fish > ~/.config/fish/completions/stout.fish
```

## Quick Start

```bash
# Update the formula index
stout update

# Search for packages
stout search json

# Get package info
stout info jq

# Install a package
stout install jq

# List installed packages
stout list

# Check system health
stout doctor
```

### Cask Support (Applications)

stout can also install macOS applications (casks) and Linux apps (AppImage, Flatpak):

```bash
# Search for applications
stout cask search firefox

# Install an application
stout cask install firefox

# List installed applications
stout cask list

# Get application info
stout cask info visual-studio-code

# Uninstall an application
stout cask uninstall firefox

# Upgrade applications
stout cask upgrade
```

## Commands

stout implements 35+ commands with full Homebrew CLI compatibility:

### Package Management

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout install <pkg>` | `brew install` | Install packages |
| `stout uninstall <pkg>` | `brew uninstall` | Uninstall packages |
| `stout reinstall <pkg>` | `brew reinstall` | Reinstall packages |
| `stout upgrade [pkg]` | `brew upgrade` | Upgrade installed packages |
| `stout update` | `brew update` | Update the formula index |
| `stout outdated` | `brew outdated` | Show packages with available updates |
| `stout autoremove` | `brew autoremove` | Remove unused dependencies |

### Discovery & Information

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout search <query>` | `brew search` | Search for packages |
| `stout info <pkg>` | `brew info` | Show package information |
| `stout list` | `brew list` | List installed packages |
| `stout deps <pkg>` | `brew deps` | Show package dependencies |
| `stout uses <pkg>` | `brew uses` | Show packages that depend on a package |
| `stout why <pkg>` | `brew why` | Show why a package is installed |
| `stout home <pkg>` | `brew home` | Open package homepage in browser |

### Package Control

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout pin <pkg>` | `brew pin` | Pin packages to prevent upgrades |
| `stout unpin <pkg>` | `brew unpin` | Unpin packages to allow upgrades |
| `stout link <pkg>` | `brew link` | Create symlinks for a package |
| `stout unlink <pkg>` | `brew unlink` | Remove symlinks (keep package) |
| `stout switch <pkg> <ver>` | `brew switch` | Switch between installed versions |
| `stout rollback <pkg>` | - | Rollback to previous version |

### System & Maintenance

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout cleanup` | `brew cleanup` | Remove old downloads and cache files |
| `stout doctor` | `brew doctor` | Check system health |
| `stout config` | `brew config` | Show stout configuration |
| `stout services` | `brew services` | Manage background services |
| `stout tap` | `brew tap` | Manage custom formula repositories |
| `stout lock` | - | Manage lockfiles for reproducible environments |
| `stout history [pkg]` | - | Show package version history |
| `stout completions <shell>` | - | Generate shell completions |

### Cask (Application) Management

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout cask install <app>` | `brew install --cask` | Install applications |
| `stout cask uninstall <app>` | `brew uninstall --cask` | Uninstall applications |
| `stout cask search <query>` | `brew search --cask` | Search for applications |
| `stout cask info <app>` | `brew info --cask` | Show application information |
| `stout cask list` | `brew list --cask` | List installed applications |
| `stout cask outdated` | `brew outdated --cask` | Show outdated applications |
| `stout cask upgrade [app]` | `brew upgrade --cask` | Upgrade applications |

### Bundle & Snapshot

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout bundle` | `brew bundle` | Install from Brewfile |
| `stout bundle dump` | `brew bundle dump` | Generate Brewfile from installed |
| `stout bundle check` | `brew bundle check` | Check if Brewfile satisfied |
| `stout bundle list` | `brew bundle list` | List Brewfile entries |
| `stout bundle cleanup` | `brew bundle cleanup` | Remove packages not in Brewfile |
| `stout snapshot create <name>` | - | Create named snapshot |
| `stout snapshot list` | - | List all snapshots |
| `stout snapshot restore <name>` | - | Restore snapshot |

### Security & Audit

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout audit` | `brew audit` | Scan packages for known vulnerabilities |
| `stout audit <pkg>` | - | Audit specific package |
| `stout audit --update` | - | Update vulnerability database |

### Offline & Mirroring

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout mirror create <dir> <pkgs>` | - | Create offline mirror |
| `stout mirror serve <dir>` | - | Serve mirror via HTTP |
| `stout mirror info <dir>` | - | Show mirror information |
| `stout mirror verify <dir>` | - | Verify mirror integrity |

### Developer Tools

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `stout install -s --jobs=8` | `brew install -s` | Build from source with parallel jobs |
| `stout install -s --cc=clang` | `brew install --cc` | Build with custom compiler |
| `stout bottle create <pkg>` | `brew bottle` | Create bottle from installed package |
| `stout create <url>` | `brew create` | Create formula from URL |
| `stout create --cask <url>` | `brew create --cask` | Create cask from URL |
| `stout test <pkg>` | `brew test` | Test installed packages |
| `stout analytics on\|off\|status` | `brew analytics` | Manage opt-in analytics |

### Multi-prefix Support

stout supports multiple installation prefixes for isolated environments:

| Command | Description |
|---------|-------------|
| `stout prefix create <path>` | Create a new prefix directory structure |
| `stout prefix list` | List all known prefixes |
| `stout prefix info [path]` | Show prefix information and disk usage |
| `stout prefix default <path>` | Set the default prefix |
| `stout prefix remove <path>` | Remove a prefix |
| `stout --prefix=<path> install <pkg>` | Install to a specific prefix |
| `stout --prefix=<path> list` | List packages in a specific prefix |

#### Example: Project-specific Dependencies

```bash
# Create an isolated prefix for a project
stout prefix create ~/projects/myapp/.stout

# Install packages to that prefix
stout --prefix=~/projects/myapp/.stout install python@3.11 node@20

# Add to project-specific PATH
export PATH="$HOME/projects/myapp/.stout/bin:$PATH"

# List packages in the prefix
stout --prefix=~/projects/myapp/.stout list
```

#### Environment Variable

You can also set the default prefix via environment variable:

```bash
export STOUT_PREFIX=~/projects/myapp/.stout
stout install jq  # Installs to custom prefix
```

## How It Works

stout uses a hybrid architecture:

1. **SQLite Index** (~3MB): Contains formula metadata for fast queries
2. **Compressed JSON**: Individual formula details fetched on-demand
3. **Homebrew Bottles**: Uses existing Homebrew bottle infrastructure

```
┌─────────────────────────────────────────────────────────────────┐
│                         stout Architecture                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   User ──▶ stout CLI ──▶ SQLite Index ──▶ Homebrew Bottles      │
│                │              │                   │              │
│                │         FTS5 Search         ghcr.io CDN        │
│                │              │                   │              │
│                └──────────────┴───────────────────┘              │
│                                                                  │
│   Index sync: GitHub raw (stout-index repo)                      │
│   Bottle download: Homebrew's existing infrastructure            │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Compatibility

stout is designed to work alongside existing Homebrew installations:

- Uses the same Cellar structure (`/opt/homebrew/Cellar`)
- Creates compatible `INSTALL_RECEIPT.json` files
- Symlinks to the same prefix (`/opt/homebrew/bin`, etc.)
- Packages installed by either tool are visible to both

## Configuration

Configuration is stored in `~/.stout/config.toml`:

```toml
[index]
base_url = "https://raw.githubusercontent.com/neul-labs/stout-index/main"
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
stout/
├── src/                    # CLI application
│   ├── main.rs
│   └── cli/               # Command implementations
├── crates/
│   ├── stout-index/       # SQLite index management
│   ├── stout-resolve/     # Dependency resolution
│   ├── stout-fetch/       # Download management
│   ├── stout-install/     # Package installation
│   ├── stout-state/       # Local state management
│   ├── stout-cask/        # Cask (application) management
│   ├── stout-bundle/      # Brewfile parsing and snapshots
│   ├── stout-audit/       # Vulnerability auditing
│   └── stout-mirror/      # Offline mirror support
├── scripts/
│   ├── sync.py            # Formula index sync script
│   ├── sync_casks.py      # Cask index sync script
│   ├── sync_linux_apps.py # Linux apps sync script
│   └── sync_vulns.py      # Vulnerability index sync script
├── packaging/             # Package manager distribution
│   ├── homebrew/          # Homebrew formula
│   ├── aur/               # Arch Linux PKGBUILD
│   └── nix/               # Nix flake
├── completions/           # Shell completions
└── docs/                  # Documentation
```

## Security

stout implements a defense-in-depth security model:

- **Ed25519 Signatures**: All index updates are cryptographically signed
- **HTTPS Required**: TLS 1.2+ enforced for all connections
- **SHA256 Verification**: Every download is checksum-verified
- **Vulnerability Scanning**: Built-in `stout audit` command

```bash
# Check security configuration
stout config

# Scan for vulnerabilities
stout audit

# Update vulnerability database
stout audit --update
```

See [Security Model](docs/SECURITY.md) for full details.

## Enterprise Features

stout is designed for enterprise environments:

- **Private Index Hosting**: Host your own curated package index
- **Custom Signing Keys**: Use your own Ed25519 keys for trust chain control
- **Air-Gapped Support**: Full offline operation with mirror support
- **CI/CD Integration**: GitHub Actions, GitLab CI, Jenkins examples
- **Multi-Prefix**: Isolated environments per project or team
- **Audit Logging**: Track all package operations for compliance

```bash
# Create offline mirror
stout mirror create /path/to/mirror jq curl python@3.11

# Project-specific prefix
stout prefix create ~/project/.stout
stout --prefix=~/project/.stout install node@20

# Reproducible builds with lock files
stout lock generate
stout lock install
```

See [Enterprise Guide](docs/ENTERPRISE.md) for deployment options.

## Documentation

- [Installation Guide](docs/INSTALL.md)
- [Usage Guide](docs/USAGE.md)
- [Security Model](docs/SECURITY.md)
- [Enterprise Guide](docs/ENTERPRISE.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Technical Specification](docs/SPEC.md)
- [Contributing](docs/CONTRIBUTING.md)
- [Packaging Guide](docs/PACKAGING.md)
- [Roadmap](docs/ROADMAP.md)

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

stout achieves its speed through several optimizations:

1. **Native binary**: No interpreter startup overhead
2. **SQLite + FTS5**: Instant full-text search
3. **Compressed index**: ~3MB download vs 700MB+ git repo
4. **Parallel downloads**: Concurrent bottle fetching with Tokio
5. **On-demand fetching**: Only download what you need

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [Homebrew](https://brew.sh) - The package manager stout is compatible with
- [uv](https://github.com/astral-sh/uv) - Inspiration for CLI UX design
