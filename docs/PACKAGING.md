# Packaging Guide

This guide covers how to package and distribute brewx through various package managers.

## Overview

brewx is distributed through multiple channels:

| Channel | Method | Target Users |
|---------|--------|--------------|
| GitHub Releases | Binary downloads | All users |
| curl installer | Automated install | Quick setup |
| Homebrew | `brew install` | macOS/Linux users |
| AUR | `yay -S brewx` | Arch Linux users |
| Nix | `nix run` | NixOS users |
| crates.io | `cargo install` | Rust developers |

## GitHub Releases

### Automated Release Process

When a tag is pushed (e.g., `v0.1.0`), the release workflow:

1. **Builds binaries** for all platforms:
   - `x86_64-unknown-linux-gnu` (Linux x86_64, glibc)
   - `x86_64-unknown-linux-musl` (Linux x86_64, musl)
   - `aarch64-unknown-linux-gnu` (Linux ARM64)
   - `x86_64-apple-darwin` (macOS Intel)
   - `aarch64-apple-darwin` (macOS Apple Silicon)

2. **Creates archives** and checksums:
   - `brewx-<target>.tar.gz`
   - `brewx-<target>.tar.gz.sha256`

3. **Publishes release** with all artifacts

### Creating a Release

```bash
# 1. Update version
# Edit Cargo.toml files to bump version

# 2. Commit changes
git add -A
git commit -m "Release v0.2.0"

# 3. Create tag
git tag v0.2.0

# 4. Push (triggers release workflow)
git push origin main --tags
```

## curl Installer

The `install.sh` script provides one-command installation:

```bash
curl -fsSL https://raw.githubusercontent.com/anthropics/brewx/main/install.sh | bash
```

### How It Works

1. Detects OS (`linux`/`darwin`) and architecture (`x86_64`/`aarch64`)
2. Fetches latest version from GitHub API
3. Downloads appropriate binary and checksum
4. Verifies SHA256 checksum
5. Installs to `~/.local/bin` or `/usr/local/bin`
6. Adds to PATH if needed

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `BREWX_INSTALL_DIR` | Installation directory | `~/.local/bin` |
| `BREWX_VERSION` | Specific version | Latest |
| `BREWX_NO_MODIFY_PATH` | Skip PATH modification | 0 |

## Homebrew

### Tap Setup

1. Create `anthropics/homebrew-tap` repository
2. Add `Formula/brewx.rb`
3. Users install via `brew install anthropics/tap/brewx`

### Formula Structure

```ruby
class Brewx < Formula
  desc "Fast, Rust-based Homebrew-compatible package manager"
  homepage "https://github.com/anthropics/brewx"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/.../brewx-aarch64-apple-darwin.tar.gz"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/.../brewx-x86_64-apple-darwin.tar.gz"
      sha256 "..."
    end
  end

  on_linux do
    # Similar structure
  end

  def install
    bin.install "brewx"
    generate_completions_from_executable(bin/"brewx", "completions")
  end

  test do
    assert_match "brewx #{version}", shell_output("#{bin}/brewx --version")
  end
end
```

### Updating After Release

```bash
cd packaging/homebrew
./update-formula.sh 0.2.0
# Copy to tap repo and push
```

## Arch User Repository (AUR)

### Packages

- `brewx` - Build from source
- `brewx-bin` - Pre-built binary

### Publishing to AUR

```bash
# Clone AUR repo
git clone ssh://aur@aur.archlinux.org/brewx.git

# Update PKGBUILD
cd brewx
vim PKGBUILD  # Update version, checksums

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.2.0"
git push
```

### Testing Locally

```bash
# Build package
makepkg -f

# Install
makepkg -si

# Check with namcap
namcap PKGBUILD
namcap brewx-0.2.0-1-x86_64.pkg.tar.zst
```

## Nix

### Flake Usage

```bash
# Run without installing
nix run github:anthropics/brewx

# Install to profile
nix profile install github:anthropics/brewx

# Development shell
nix develop github:anthropics/brewx
```

### Adding to nixpkgs

Submit PR to nixpkgs with package in `pkgs/by-name/br/brewx/package.nix`.

## crates.io

### Publishing

The release workflow automatically publishes to crates.io after creating the GitHub release.

Crates are published in dependency order:
1. `brewx-index`
2. `brewx-state`
3. `brewx-resolve`
4. `brewx-fetch`
5. `brewx-install`
6. `brewx`

### Manual Publishing

```bash
# Login (one-time)
cargo login

# Publish in order
cargo publish -p brewx-index
cargo publish -p brewx-state
cargo publish -p brewx-resolve
cargo publish -p brewx-fetch
cargo publish -p brewx-install
cargo publish
```

### Requirements

Each crate's `Cargo.toml` must have:
- `version` - Semantic version
- `description` - Short description
- `license` - License identifier
- `repository` - GitHub URL
- `readme` - README file path (optional)

## Version Management

### Versioning Strategy

We use [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes, backward compatible

### Updating Versions

All `Cargo.toml` files must have matching versions:

```bash
# Use cargo-edit for batch updates
cargo install cargo-edit
cargo set-version 0.2.0
```

Or manually update each:
- `Cargo.toml` (root)
- `crates/brewx-index/Cargo.toml`
- `crates/brewx-state/Cargo.toml`
- `crates/brewx-resolve/Cargo.toml`
- `crates/brewx-fetch/Cargo.toml`
- `crates/brewx-install/Cargo.toml`

## Secrets Required

GitHub repository secrets needed for automated releases:

| Secret | Purpose |
|--------|---------|
| `GITHUB_TOKEN` | Auto-provided, release creation |
| `CARGO_REGISTRY_TOKEN` | crates.io publishing |
| `TAP_GITHUB_TOKEN` | Homebrew tap updates (optional) |

### Setting Up Secrets

1. **CARGO_REGISTRY_TOKEN**:
   - Go to https://crates.io/settings/tokens
   - Create new token with publish scope
   - Add to GitHub repo secrets

2. **TAP_GITHUB_TOKEN** (optional):
   - Create PAT with `repo` scope
   - Add to GitHub repo secrets

## Troubleshooting

### crates.io Publish Fails

- Check version not already published
- Ensure all dependencies are published
- Wait 30s between publishes for indexing

### Homebrew Formula Issues

```bash
# Audit formula
brew audit --strict brewx

# Test installation
brew install --build-from-source ./brewx.rb
brew test brewx
```

### AUR Build Fails

```bash
# Build in clean chroot
extra-x86_64-build

# Check dependencies
namcap PKGBUILD
```
