# Installation Guide

This guide covers all methods for installing brewx.

## Requirements

- **Operating System**: macOS (ARM or Intel) or Linux
- **Rust**: 1.75+ (for building from source only)
- **Homebrew**: Optional, but required for installing packages

## Quick Install (Recommended)

The easiest way to install brewx is using the automatic installer:

```bash
curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash
```

This script will:
1. Detect your operating system (macOS or Linux)
2. Detect your CPU architecture (x86_64 or ARM64)
3. Download the appropriate pre-built binary
4. Verify the SHA256 checksum
5. Install to `~/.local/bin` or `/usr/local/bin`
6. Add to your PATH if needed

### Installation Options

You can customize the installation using environment variables:

```bash
# Install to a custom directory
BREWX_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash

# Install a specific version
BREWX_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash

# Skip automatic PATH modification
BREWX_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash
```

## Manual Download

If you prefer to download manually, get the appropriate binary from the [releases page](https://github.com/anthropics/brewx/releases):

| Platform | Architecture | Binary |
|----------|-------------|--------|
| macOS | Apple Silicon (M1/M2/M3) | `brewx-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `brewx-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 (glibc) | `brewx-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | x86_64 (musl) | `brewx-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `brewx-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example: macOS Apple Silicon
curl -LO https://github.com/anthropics/brewx/releases/latest/download/brewx-aarch64-apple-darwin.tar.gz
tar -xzf brewx-aarch64-apple-darwin.tar.gz
sudo mv brewx /usr/local/bin/

# Example: Linux x86_64
curl -LO https://github.com/anthropics/brewx/releases/latest/download/brewx-x86_64-unknown-linux-gnu.tar.gz
tar -xzf brewx-x86_64-unknown-linux-gnu.tar.gz
sudo mv brewx /usr/local/bin/
```

### Verifying the Download

Each release includes SHA256 checksums:

```bash
# Download checksum file
curl -LO https://github.com/anthropics/brewx/releases/latest/download/brewx-aarch64-apple-darwin.tar.gz.sha256

# Verify (macOS)
shasum -a 256 -c brewx-aarch64-apple-darwin.tar.gz.sha256

# Verify (Linux)
sha256sum -c brewx-aarch64-apple-darwin.tar.gz.sha256
```

## From Source

```bash
# Clone the repository
git clone https://github.com/anthropics/brewx.git
cd brewx

# Build release binary (optimized)
cargo build --release

# Install to ~/.local/bin (user-local)
mkdir -p ~/.local/bin
cp target/release/brewx ~/.local/bin/

# Or install system-wide
sudo cp target/release/brewx /usr/local/bin/
```

### Using Cargo

```bash
cargo install --git https://github.com/anthropics/brewx.git
```

## Post-Installation Setup

### 1. Initialize brewx

After installation, initialize brewx and download the formula index:

```bash
# Check installation
brewx --version

# Download the formula index (required for first use)
brewx update

# Verify installation
brewx doctor
```

### 2. Shell Completions

Enable tab completion for your shell:

#### Bash

Add to `~/.bashrc`:

```bash
eval "$(brewx completions bash)"
```

Or install permanently:

```bash
brewx completions bash > /etc/bash_completion.d/brewx
```

#### Zsh

Add to `~/.zshrc`:

```bash
eval "$(brewx completions zsh)"
```

Or install to your fpath:

```bash
brewx completions zsh > ~/.zsh/completions/_brewx
# Ensure ~/.zsh/completions is in your fpath
```

#### Fish

```bash
brewx completions fish > ~/.config/fish/completions/brewx.fish
```

### 3. Configuration (Optional)

Create a configuration file at `~/.brewx/config.toml`:

```toml
[index]
# Use custom index URL (default: anthropics/brewx-index)
base_url = "https://raw.githubusercontent.com/anthropics/brewx-index/main"

# Auto-update index when stale
auto_update = true

# Update check interval in seconds (default: 30 minutes)
update_interval = 1800

[install]
# Homebrew Cellar path
cellar = "/opt/homebrew/Cellar"

# Homebrew prefix
prefix = "/opt/homebrew"

# Concurrent downloads (default: 4)
parallel_downloads = 4

[cache]
# Maximum cache size
max_size = "2GB"

# Formula cache TTL in seconds (default: 1 day)
formula_ttl = 86400

# Download cache TTL in seconds (default: 7 days)
download_ttl = 604800
```

## Platform-Specific Notes

### macOS

brewx auto-detects your Homebrew installation:
- **Apple Silicon (M1/M2/M3)**: `/opt/homebrew`
- **Intel**: `/usr/local`

### Linux

brewx looks for Homebrew in:
1. `/home/linuxbrew/.linuxbrew`
2. `/usr/local`

If you don't have Homebrew installed, brewx will still work for searching and viewing package info, but you won't be able to install packages.

## Verifying Installation

Run the doctor command to verify everything is set up correctly:

```bash
brewx doctor
```

Expected output:

```
brewx doctor
Checking system health...

  Checking brewx directory... ✓
  Checking configuration... ✓
  Checking formula index... ✓ (8058 formulas)
  Checking Homebrew prefix... ✓
  Checking Cellar... ✓
  Checking installed packages state... ✓ (N tracked)

Your system is ready to brew!
```

## Updating brewx

### From Source

```bash
cd brewx
git pull
cargo build --release
cp target/release/brewx ~/.local/bin/
```

### Using Cargo

```bash
cargo install --git https://github.com/anthropics/brewx.git --force
```

## Uninstalling

```bash
# Remove binary
rm ~/.local/bin/brewx
# or
sudo rm /usr/local/bin/brewx

# Remove data directory (optional)
rm -rf ~/.brewx

# Remove shell completions
# (location depends on how you installed them)
```

## Troubleshooting

### "Index not initialized"

Run `brewx update` to download the formula index.

### "Homebrew prefix not found"

Install Homebrew first, or configure a custom prefix in `~/.brewx/config.toml`.

### "Permission denied"

Ensure you have write access to the Homebrew Cellar directory, or use `sudo`.

### Build fails with Rust errors

Ensure you have Rust 1.75 or later:

```bash
rustup update stable
```
