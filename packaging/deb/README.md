# Debian Packaging for stout

This directory contains documentation for building Debian packages (.deb) for stout.

## Automated Builds

Debian packages are automatically built and published to GitHub Releases when a new version tag is pushed. The release workflow builds packages for:

- `amd64` (x86_64)
- `arm64` (aarch64)

## Manual Build

To build a .deb package locally:

```bash
# Install cargo-deb
cargo install cargo-deb

# Build the release binary with man pages
STOUT_GEN_MAN=1 cargo build --release

# Generate shell completions
mkdir -p target/completions
./target/release/stout completions bash > target/completions/stout.bash
./target/release/stout completions zsh > target/completions/_stout
./target/release/stout completions fish > target/completions/stout.fish

# Build the .deb package
cargo deb
```

The resulting `.deb` file will be in `target/debian/`.

## Installation

```bash
# Download from GitHub releases
wget https://github.com/neul-labs/stout/releases/latest/download/stout_VERSION_amd64.deb

# Install
sudo dpkg -i stout_VERSION_amd64.deb

# Or install with apt to handle dependencies
sudo apt install ./stout_VERSION_amd64.deb
```

## Package Contents

The .deb package includes:
- `/usr/bin/stout` - The main binary
- `/usr/share/doc/stout/` - Documentation (README, LICENSE)
- `/usr/share/man/man1/stout.1` - Man page
- `/usr/share/bash-completion/completions/stout` - Bash completions
- `/usr/share/fish/vendor_completions.d/stout.fish` - Fish completions
- `/usr/share/zsh/vendor-completions/_stout` - Zsh completions

## Configuration

The packaging configuration is in the root `Cargo.toml` under `[package.metadata.deb]`.
