# brewx

A fast, Rust-based Homebrew-compatible package manager.

[![Build](https://github.com/neul-labs/brewx/actions/workflows/ci.yml/badge.svg)](https://github.com/neul-labs/brewx/actions/workflows/ci.yml)
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
curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash
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
BREWX_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash

# Install a specific version
BREWX_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash

# Skip PATH modification
BREWX_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash
```

### Manual Download

Download pre-built binaries from the [releases page](https://github.com/neul-labs/brewx/releases):

| Platform | Architecture | Download |
|----------|-------------|----------|
| macOS | Apple Silicon (M1/M2/M3) | `brewx-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `brewx-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 | `brewx-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | ARM64 | `brewx-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example for macOS ARM
curl -LO https://github.com/neul-labs/brewx/releases/latest/download/brewx-aarch64-apple-darwin.tar.gz
tar -xzf brewx-aarch64-apple-darwin.tar.gz
sudo mv brewx /usr/local/bin/
```

### From Source

```bash
# Clone the repository
git clone https://github.com/neul-labs/brewx.git
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

### Cask Support (Applications)

brewx can also install macOS applications (casks) and Linux apps (AppImage, Flatpak):

```bash
# Search for applications
brewx cask search firefox

# Install an application
brewx cask install firefox

# List installed applications
brewx cask list

# Get application info
brewx cask info visual-studio-code

# Uninstall an application
brewx cask uninstall firefox

# Upgrade applications
brewx cask upgrade
```

## Commands

brewx implements 35+ commands with full Homebrew CLI compatibility:

### Package Management

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx install <pkg>` | `brew install` | Install packages |
| `brewx uninstall <pkg>` | `brew uninstall` | Uninstall packages |
| `brewx reinstall <pkg>` | `brew reinstall` | Reinstall packages |
| `brewx upgrade [pkg]` | `brew upgrade` | Upgrade installed packages |
| `brewx update` | `brew update` | Update the formula index |
| `brewx outdated` | `brew outdated` | Show packages with available updates |
| `brewx autoremove` | `brew autoremove` | Remove unused dependencies |

### Discovery & Information

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx search <query>` | `brew search` | Search for packages |
| `brewx info <pkg>` | `brew info` | Show package information |
| `brewx list` | `brew list` | List installed packages |
| `brewx deps <pkg>` | `brew deps` | Show package dependencies |
| `brewx uses <pkg>` | `brew uses` | Show packages that depend on a package |
| `brewx why <pkg>` | `brew why` | Show why a package is installed |
| `brewx home <pkg>` | `brew home` | Open package homepage in browser |

### Package Control

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx pin <pkg>` | `brew pin` | Pin packages to prevent upgrades |
| `brewx unpin <pkg>` | `brew unpin` | Unpin packages to allow upgrades |
| `brewx link <pkg>` | `brew link` | Create symlinks for a package |
| `brewx unlink <pkg>` | `brew unlink` | Remove symlinks (keep package) |
| `brewx switch <pkg> <ver>` | `brew switch` | Switch between installed versions |
| `brewx rollback <pkg>` | - | Rollback to previous version |

### System & Maintenance

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx cleanup` | `brew cleanup` | Remove old downloads and cache files |
| `brewx doctor` | `brew doctor` | Check system health |
| `brewx config` | `brew config` | Show brewx configuration |
| `brewx services` | `brew services` | Manage background services |
| `brewx tap` | `brew tap` | Manage custom formula repositories |
| `brewx lock` | - | Manage lockfiles for reproducible environments |
| `brewx history [pkg]` | - | Show package version history |
| `brewx completions <shell>` | - | Generate shell completions |

### Cask (Application) Management

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx cask install <app>` | `brew install --cask` | Install applications |
| `brewx cask uninstall <app>` | `brew uninstall --cask` | Uninstall applications |
| `brewx cask search <query>` | `brew search --cask` | Search for applications |
| `brewx cask info <app>` | `brew info --cask` | Show application information |
| `brewx cask list` | `brew list --cask` | List installed applications |
| `brewx cask outdated` | `brew outdated --cask` | Show outdated applications |
| `brewx cask upgrade [app]` | `brew upgrade --cask` | Upgrade applications |

### Bundle & Snapshot

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx bundle` | `brew bundle` | Install from Brewfile |
| `brewx bundle dump` | `brew bundle dump` | Generate Brewfile from installed |
| `brewx bundle check` | `brew bundle check` | Check if Brewfile satisfied |
| `brewx bundle list` | `brew bundle list` | List Brewfile entries |
| `brewx bundle cleanup` | `brew bundle cleanup` | Remove packages not in Brewfile |
| `brewx snapshot create <name>` | - | Create named snapshot |
| `brewx snapshot list` | - | List all snapshots |
| `brewx snapshot restore <name>` | - | Restore snapshot |

### Security & Audit

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx audit` | `brew audit` | Scan packages for known vulnerabilities |
| `brewx audit <pkg>` | - | Audit specific package |
| `brewx audit --update` | - | Update vulnerability database |

### Offline & Mirroring

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx mirror create <dir> <pkgs>` | - | Create offline mirror |
| `brewx mirror serve <dir>` | - | Serve mirror via HTTP |
| `brewx mirror info <dir>` | - | Show mirror information |
| `brewx mirror verify <dir>` | - | Verify mirror integrity |

### Developer Tools

| Command | brew Equivalent | Description |
|---------|-----------------|-------------|
| `brewx install -s --jobs=8` | `brew install -s` | Build from source with parallel jobs |
| `brewx install -s --cc=clang` | `brew install --cc` | Build with custom compiler |
| `brewx bottle create <pkg>` | `brew bottle` | Create bottle from installed package |
| `brewx create <url>` | `brew create` | Create formula from URL |
| `brewx create --cask <url>` | `brew create --cask` | Create cask from URL |
| `brewx test <pkg>` | `brew test` | Test installed packages |
| `brewx analytics on\|off\|status` | `brew analytics` | Manage opt-in analytics |

### Multi-prefix Support

brewx supports multiple installation prefixes for isolated environments:

| Command | Description |
|---------|-------------|
| `brewx prefix create <path>` | Create a new prefix directory structure |
| `brewx prefix list` | List all known prefixes |
| `brewx prefix info [path]` | Show prefix information and disk usage |
| `brewx prefix default <path>` | Set the default prefix |
| `brewx prefix remove <path>` | Remove a prefix |
| `brewx --prefix=<path> install <pkg>` | Install to a specific prefix |
| `brewx --prefix=<path> list` | List packages in a specific prefix |

#### Example: Project-specific Dependencies

```bash
# Create an isolated prefix for a project
brewx prefix create ~/projects/myapp/.brewx

# Install packages to that prefix
brewx --prefix=~/projects/myapp/.brewx install python@3.11 node@20

# Add to project-specific PATH
export PATH="$HOME/projects/myapp/.brewx/bin:$PATH"

# List packages in the prefix
brewx --prefix=~/projects/myapp/.brewx list
```

#### Environment Variable

You can also set the default prefix via environment variable:

```bash
export BREWX_PREFIX=~/projects/myapp/.brewx
brewx install jq  # Installs to custom prefix
```

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
base_url = "https://raw.githubusercontent.com/neul-labs/brewx-index/main"
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
│   ├── brewx-state/       # Local state management
│   ├── brewx-cask/        # Cask (application) management
│   ├── brewx-bundle/      # Brewfile parsing and snapshots
│   ├── brewx-audit/       # Vulnerability auditing
│   └── brewx-mirror/      # Offline mirror support
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

brewx implements a defense-in-depth security model:

- **Ed25519 Signatures**: All index updates are cryptographically signed
- **HTTPS Required**: TLS 1.2+ enforced for all connections
- **SHA256 Verification**: Every download is checksum-verified
- **Vulnerability Scanning**: Built-in `brewx audit` command

```bash
# Check security configuration
brewx config

# Scan for vulnerabilities
brewx audit

# Update vulnerability database
brewx audit --update
```

See [Security Model](docs/SECURITY.md) for full details.

## Enterprise Features

brewx is designed for enterprise environments:

- **Private Index Hosting**: Host your own curated package index
- **Custom Signing Keys**: Use your own Ed25519 keys for trust chain control
- **Air-Gapped Support**: Full offline operation with mirror support
- **CI/CD Integration**: GitHub Actions, GitLab CI, Jenkins examples
- **Multi-Prefix**: Isolated environments per project or team
- **Audit Logging**: Track all package operations for compliance

```bash
# Create offline mirror
brewx mirror create /path/to/mirror jq curl python@3.11

# Project-specific prefix
brewx prefix create ~/project/.brewx
brewx --prefix=~/project/.brewx install node@20

# Reproducible builds with lock files
brewx lock generate
brewx lock install
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
