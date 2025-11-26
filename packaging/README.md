# Package Distribution

This directory contains packaging files for distributing brewx through various package managers.

## Available Packages

| Package Manager | Platform | Install Command |
|-----------------|----------|-----------------|
| **curl installer** | All | `curl -fsSL https://...install.sh \| bash` |
| **Homebrew** | macOS, Linux | `brew install anthropics/tap/brewx` |
| **AUR** | Arch Linux | `yay -S brewx` |
| **Nix** | NixOS, Any | `nix run github:anthropics/brewx` |
| **Cargo** | Any | `cargo install brewx` |

## Directory Structure

```
packaging/
├── README.md           # This file
├── homebrew/
│   ├── brewx.rb        # Homebrew formula
│   ├── update-formula.sh
│   └── README.md
├── aur/
│   ├── PKGBUILD        # Source package
│   ├── PKGBUILD-bin    # Binary package
│   └── README.md
├── nix/
│   ├── default.nix     # Nix package
│   ├── flake.nix       # Flakes support
│   └── README.md
├── deb/                # Debian package (future)
└── rpm/                # RPM package (future)
```

## Release Checklist

When creating a new release:

### 1. Pre-release

```bash
# Update version in Cargo.toml files
# Update CHANGELOG.md
# Commit and tag
git tag v0.2.0
git push origin v0.2.0
```

### 2. Automatic (via GitHub Actions)

The release workflow automatically:
- Builds binaries for all platforms
- Creates SHA256 checksums
- Publishes to GitHub releases
- Publishes to crates.io

### 3. Manual Updates Required

After the release is published:

#### Homebrew

```bash
cd packaging/homebrew
./update-formula.sh 0.2.0
# Copy to tap repository and push
```

#### AUR

```bash
cd packaging/aur
# Update pkgver and checksums
makepkg --printsrcinfo > .SRCINFO
# Push to AUR
```

#### Nix

```bash
# Update version and hash in default.nix
# Submit PR to nixpkgs (optional)
```

## Installation Priority

For users, we recommend installation in this order:

1. **Homebrew** (macOS users) - Most familiar
2. **AUR** (Arch Linux users) - Integrates with system
3. **Nix** (NixOS users) - Reproducible
4. **curl installer** - Universal, no prerequisites
5. **Cargo** - For Rust developers

## Future Packages

Planned but not yet implemented:

- **Debian/Ubuntu** (.deb) - `apt install brewx`
- **Fedora/RHEL** (.rpm) - `dnf install brewx`
- **Snap** - `snap install brewx`
- **Flatpak** - `flatpak install brewx`
- **Chocolatey** (Windows) - `choco install brewx`
- **Scoop** (Windows) - `scoop install brewx`

## Package Naming

- Package name: `brewx`
- Binary name: `brewx`
- Source package: `brewx` (builds from source)
- Binary package: `brewx-bin` (pre-built binary)
- Git package: `brewx-git` (builds from git HEAD)

## Maintainer Contacts

For package-specific issues:
- Homebrew: Open issue on `anthropics/homebrew-tap`
- AUR: Contact maintainer via AUR
- Nix: Open issue on main repo or nixpkgs

## License

All packaging files are released under the MIT license, same as the main project.
