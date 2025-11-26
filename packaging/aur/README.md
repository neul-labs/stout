# AUR Distribution

This directory contains PKGBUILD files for distributing brewx via the Arch User Repository (AUR).

## Packages

### brewx (source)

Build from source. Recommended for most users.

```bash
# Using yay
yay -S brewx

# Using paru
paru -S brewx

# Manual
git clone https://aur.archlinux.org/brewx.git
cd brewx
makepkg -si
```

### brewx-bin (binary)

Pre-built binary. Faster installation, no Rust toolchain needed.

```bash
# Using yay
yay -S brewx-bin

# Using paru
paru -S brewx-bin
```

## Publishing to AUR

### Initial Setup

1. Create an AUR account at https://aur.archlinux.org
2. Add your SSH public key to your AUR account
3. Clone the AUR package:

```bash
# For new package
git clone ssh://aur@aur.archlinux.org/brewx.git
cd brewx
```

### Updating the Package

1. Update `pkgver` in PKGBUILD
2. Update `sha256sums` (run `updpkgsums` or calculate manually)
3. Update `.SRCINFO`:

```bash
makepkg --printsrcinfo > .SRCINFO
```

4. Commit and push:

```bash
git add PKGBUILD .SRCINFO
git commit -m "Update to version X.Y.Z"
git push
```

### Calculating SHA256

For source package:
```bash
curl -sL "https://github.com/anthropics/brewx/archive/v0.1.0.tar.gz" | sha256sum
```

For binary package:
```bash
curl -sL "https://github.com/anthropics/brewx/releases/download/v0.1.0/brewx-x86_64-unknown-linux-gnu.tar.gz" | sha256sum
curl -sL "https://github.com/anthropics/brewx/releases/download/v0.1.0/brewx-aarch64-unknown-linux-gnu.tar.gz" | sha256sum
```

## Testing Locally

```bash
# Test building
makepkg -f

# Test installation
makepkg -si

# Check package
namcap PKGBUILD
namcap brewx-0.1.0-1-x86_64.pkg.tar.zst
```

## AUR Guidelines

- Package names should be lowercase
- Use `$pkgname` and `$pkgver` variables consistently
- Include proper `provides` and `conflicts` arrays
- Run `namcap` to check for common issues
- Test on a clean Arch installation (container/VM) before publishing

## Update Script

```bash
#!/bin/bash
# update-aur.sh - Update AUR package

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    exit 1
fi

# Update PKGBUILD
sed -i "s/pkgver=.*/pkgver=$VERSION/" PKGBUILD

# Update checksums
updpkgsums

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

echo "Updated to version $VERSION"
echo "Review changes, then: git add -A && git commit -m 'Update to $VERSION' && git push"
```
