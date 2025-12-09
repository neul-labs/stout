# Installation Guide

This guide covers all methods for installing stout.

## Requirements

- **Operating System**: macOS (ARM or Intel) or Linux
- **Rust**: 1.75+ (for building from source only)
- **Homebrew**: Optional, but required for installing packages

## Quick Install (Recommended)

The easiest way to install stout is using the automatic installer:

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
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
STOUT_INSTALL_DIR=/opt/bin curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash

# Install a specific version
STOUT_VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash

# Skip automatic PATH modification
STOUT_NO_MODIFY_PATH=1 curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

## Package Managers

### npm (Node.js)

```bash
npm install -g stout-pkg
```

### pip (Python)

```bash
pip install stout-pkg

# Or with pipx for isolated installation
pipx install stout-pkg
```

### gem (Ruby)

```bash
gem install stout-pkg
```

### Debian / Ubuntu (APT)

Download and install the `.deb` package from the [releases page](https://github.com/neul-labs/stout/releases):

```bash
# Download (replace VERSION with the actual version, e.g., 0.1.0)
wget https://github.com/neul-labs/stout/releases/latest/download/stout_VERSION_amd64.deb

# Install with apt (handles dependencies)
sudo apt install ./stout_VERSION_amd64.deb

# Or install with dpkg
sudo dpkg -i stout_VERSION_amd64.deb
```

For ARM64 systems, use `stout_VERSION_arm64.deb` instead.

### Fedora / RHEL / CentOS (DNF/RPM)

Download and install the `.rpm` package from the [releases page](https://github.com/neul-labs/stout/releases):

```bash
# Download (replace VERSION with the actual version, e.g., 0.1.0)
wget https://github.com/neul-labs/stout/releases/latest/download/stout-VERSION-1.x86_64.rpm

# Install with dnf
sudo dnf install ./stout-VERSION-1.x86_64.rpm

# Or with rpm directly
sudo rpm -i stout-VERSION-1.x86_64.rpm
```

For ARM64 systems, use `stout-VERSION-1.aarch64.rpm` instead.

### Arch Linux (AUR)

Install from the AUR using your preferred AUR helper:

```bash
# Using yay
yay -S stout

# Using paru
paru -S stout
```

### Nix / NixOS

Using flakes:

```bash
# Run directly
nix run github:neul-labs/stout

# Install to profile
nix profile install github:neul-labs/stout

# Add to your flake.nix
{
  inputs.stout.url = "github:neul-labs/stout";
}
```

### Homebrew

```bash
# Using the tap (recommended)
brew tap neul-labs/tap
brew install stout
```

## Manual Download

If you prefer to download manually, get the appropriate binary from the [releases page](https://github.com/neul-labs/stout/releases):

| Platform | Architecture | Binary |
|----------|-------------|--------|
| macOS | Apple Silicon (M1/M2/M3) | `stout-aarch64-apple-darwin.tar.gz` |
| macOS | Intel | `stout-x86_64-apple-darwin.tar.gz` |
| Linux | x86_64 (glibc) | `stout-x86_64-unknown-linux-gnu.tar.gz` |
| Linux | x86_64 (musl) | `stout-x86_64-unknown-linux-musl.tar.gz` |
| Linux | ARM64 | `stout-aarch64-unknown-linux-gnu.tar.gz` |

```bash
# Example: macOS Apple Silicon
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout-aarch64-apple-darwin.tar.gz
tar -xzf stout-aarch64-apple-darwin.tar.gz
sudo mv stout /usr/local/bin/

# Example: Linux x86_64
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout-x86_64-unknown-linux-gnu.tar.gz
tar -xzf stout-x86_64-unknown-linux-gnu.tar.gz
sudo mv stout /usr/local/bin/
```

### Verifying the Download

Each release includes SHA256 checksums:

```bash
# Download checksum file
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout-aarch64-apple-darwin.tar.gz.sha256

# Verify (macOS)
shasum -a 256 -c stout-aarch64-apple-darwin.tar.gz.sha256

# Verify (Linux)
sha256sum -c stout-aarch64-apple-darwin.tar.gz.sha256
```

## From Source

```bash
# Clone the repository
git clone https://github.com/neul-labs/stout.git
cd stout

# Build release binary (optimized)
cargo build --release

# Install to ~/.local/bin (user-local)
mkdir -p ~/.local/bin
cp target/release/stout ~/.local/bin/

# Or install system-wide
sudo cp target/release/stout /usr/local/bin/
```

### Using Cargo

```bash
cargo install --git https://github.com/neul-labs/stout.git
```

## Post-Installation Setup

### 1. Initialize stout

After installation, initialize stout and download the formula index:

```bash
# Check installation
stout --version

# Download the formula index (required for first use)
stout update

# Verify installation
stout doctor
```

### 2. Shell Completions

Enable tab completion for your shell:

#### Bash

Add to `~/.bashrc`:

```bash
eval "$(stout completions bash)"
```

Or install permanently:

```bash
stout completions bash > /etc/bash_completion.d/stout
```

#### Zsh

Add to `~/.zshrc`:

```bash
eval "$(stout completions zsh)"
```

Or install to your fpath:

```bash
stout completions zsh > ~/.zsh/completions/_stout
# Ensure ~/.zsh/completions is in your fpath
```

#### Fish

```bash
stout completions fish > ~/.config/fish/completions/stout.fish
```

### 3. Configuration (Optional)

Create a configuration file at `~/.stout/config.toml`:

```toml
[index]
# Use custom index URL (default: neul-labs/stout-index)
base_url = "https://raw.githubusercontent.com/neul-labs/stout-index/main"

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

stout auto-detects your Homebrew installation:
- **Apple Silicon (M1/M2/M3)**: `/opt/homebrew`
- **Intel**: `/usr/local`

### Linux

stout looks for Homebrew in:
1. `/home/linuxbrew/.linuxbrew`
2. `/usr/local`

If you don't have Homebrew installed, stout will still work for searching and viewing package info, but you won't be able to install packages.

## Verifying Installation

Run the doctor command to verify everything is set up correctly:

```bash
stout doctor
```

Expected output:

```
stout doctor
Checking system health...

  Checking stout directory... ✓
  Checking configuration... ✓
  Checking formula index... ✓ (8058 formulas)
  Checking Homebrew prefix... ✓
  Checking Cellar... ✓
  Checking installed packages state... ✓ (N tracked)

Your system is ready to brew!
```

## Updating stout

### From Source

```bash
cd stout
git pull
cargo build --release
cp target/release/stout ~/.local/bin/
```

### Using Cargo

```bash
cargo install --git https://github.com/neul-labs/stout.git --force
```

## Uninstalling

```bash
# Remove binary
rm ~/.local/bin/stout
# or
sudo rm /usr/local/bin/stout

# Remove data directory (optional)
rm -rf ~/.stout

# Remove shell completions
# (location depends on how you installed them)
```

## Troubleshooting

### "Index not initialized"

Run `stout update` to download the formula index.

### "Homebrew prefix not found"

Install Homebrew first, or configure a custom prefix in `~/.stout/config.toml`.

### "Permission denied"

Ensure you have write access to the Homebrew Cellar directory, or use `sudo`.

### Build fails with Rust errors

Ensure you have Rust 1.75 or later:

```bash
rustup update stable
```
