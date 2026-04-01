# Packaging Guide

This guide covers how to package and distribute stout through various package managers.

## Overview

stout is distributed through multiple channels:

| Channel | Method | Target Users |
|---------|--------|--------------|
| GitHub Releases | Binary downloads | All users |
| curl installer | Automated install | Quick setup |
| Homebrew | `brew install` | macOS/Linux users |
| AUR | `yay -S stout` | Arch Linux users |
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
   - `stout-<target>.tar.gz`
   - `stout-<target>.tar.gz.sha256`

3. **Publishes release** with all artifacts

### Creating a Release

```bash
# 1. Update version
# Edit Cargo.toml files to bump version

# 2. Commit changes
git add -A
git commit -m "Release v0.2.1"

# 3. Create tag
git tag v0.2.1

# 4. Push (triggers release workflow)
git push origin main --tags
```

## curl Installer

The `install.sh` script provides one-command installation:

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
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
| `STOUT_INSTALL_DIR` | Installation directory | `~/.local/bin` |
| `STOUT_VERSION` | Specific version | Latest |
| `STOUT_NO_MODIFY_PATH` | Skip PATH modification | 0 |

## Homebrew

### Tap Setup

1. Create `neul-labs/homebrew-tap` repository
2. Add `Formula/stout.rb`
3. Users install via `brew install neul-labs/tap/stout`

### Formula Structure

```ruby
class Stout < Formula
  desc "Fast, Rust-based Homebrew-compatible package manager"
  homepage "https://github.com/neul-labs/stout"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/.../stout-aarch64-apple-darwin.tar.gz"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/.../stout-x86_64-apple-darwin.tar.gz"
      sha256 "..."
    end
  end

  on_linux do
    # Similar structure
  end

  def install
    bin.install "stout"
    generate_completions_from_executable(bin/"stout", "completions")
  end

  test do
    assert_match "stout #{version}", shell_output("#{bin}/stout --version")
  end
end
```

### Updating After Release

```bash
cd packaging/homebrew
./update-formula.sh 0.2.1
# Copy to tap repo and push
```

## Arch User Repository (AUR)

### Packages

- `stout` - Build from source
- `stout-bin` - Pre-built binary

### Publishing to AUR

```bash
# Clone AUR repo
git clone ssh://aur@aur.archlinux.org/stout.git

# Update PKGBUILD
cd stout
vim PKGBUILD  # Update version, checksums

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit and push
git add PKGBUILD .SRCINFO
git commit -m "Update to 0.2.1"
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
namcap stout-0.2.1-1-x86_64.pkg.tar.zst
```

## Nix

### Flake Usage

```bash
# Run without installing
nix run github:neul-labs/stout

# Install to profile
nix profile install github:neul-labs/stout

# Development shell
nix develop github:neul-labs/stout
```

### Adding to nixpkgs

Submit PR to nixpkgs with package in `pkgs/by-name/br/stout/package.nix`.

## crates.io

### Publishing

The release workflow automatically publishes to crates.io after creating the GitHub release.

Crates are published in dependency order:
1. `stout-index`
2. `stout-state`
3. `stout-resolve`
4. `stout-fetch`
5. `stout-install`
6. `stout`

### Manual Publishing

```bash
# Login (one-time)
cargo login

# Publish in order
cargo publish -p stout-index
cargo publish -p stout-state
cargo publish -p stout-resolve
cargo publish -p stout-fetch
cargo publish -p stout-install
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
cargo set-version 0.2.1
```

Or manually update each:
- `Cargo.toml` (root)
- `crates/stout-index/Cargo.toml`
- `crates/stout-state/Cargo.toml`
- `crates/stout-resolve/Cargo.toml`
- `crates/stout-fetch/Cargo.toml`
- `crates/stout-install/Cargo.toml`

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
brew audit --strict stout

# Test installation
brew install --build-from-source ./stout.rb
brew test stout
```

### AUR Build Fails

```bash
# Build in clean chroot
extra-x86_64-build

# Check dependencies
namcap PKGBUILD
```
