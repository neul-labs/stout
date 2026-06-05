# Quick Start

Get up and running with Stout in just a few minutes.

---

## First Steps

### Update the Package Index

Before installing packages, update the formula index:

```bash
stout update
```

This downloads the pre-computed package index (about 2MB) - much faster than Homebrew's git-based update.

### Search for Packages

Find packages with full-text search:

```bash
stout search json
```

Output:
```
jq                          Lightweight and flexible command-line JSON processor
python-yq                   Command-line YAML and XML processor using jq
gron                        Make JSON greppable
fx                          Terminal JSON viewer
...
```

### Get Package Information

View details about a package:

```bash
stout info jq
```

Output:
```
jq 1.7.1
Lightweight and flexible command-line JSON processor

Homepage: https://jqlang.github.io/jq/
License: MIT

Installed: No
```

---

## Installing Packages

### Basic Installation

```bash
stout install jq
```

### Install Multiple Packages

```bash
stout install jq yq gron
```

### Install Specific Version

```bash
stout install python@3.11
```

### Build from Source

If a pre-built bottle isn't available:

```bash
stout install --build-from-source mypackage
```

---

## Managing Installed Packages

### List Installed Packages

```bash
stout list
```

### Check for Updates

```bash
stout outdated
```

### Upgrade Packages

Upgrade a specific package:

```bash
stout upgrade jq
```

Upgrade all packages:

```bash
stout upgrade
```

### Uninstall Packages

```bash
stout uninstall jq
```

### Remove Unused Dependencies

```bash
stout autoremove
```

---

## Exploring Dependencies

### View Dependencies

See what a package depends on:

```bash
stout deps python@3.11
```

### View as Tree

```bash
stout deps --tree python@3.11
```

### Reverse Dependencies

See what depends on a package:

```bash
stout uses openssl
```

### Why Is This Installed?

Trace why a package was installed:

```bash
stout why readline
```

---

## Working with Casks (Applications)

Casks are macOS applications (DMG, PKG) and Linux apps (AppImage, Flatpak).

### Search for Applications

```bash
stout cask search firefox
```

### Install an Application

```bash
stout cask install firefox
```

### List Installed Applications

```bash
stout cask list
```

### Upgrade Applications

```bash
stout cask upgrade
```

---

## System Maintenance

### Health Check

Run diagnostics:

```bash
stout doctor
```

### Clean Up

Remove old downloads and cache:

```bash
stout cleanup
```

Preview what would be removed:

```bash
stout cleanup --dry-run
```

### View Configuration

```bash
stout config
```

---

## Helpful Tips

### Pin Packages

Prevent a package from being upgraded:

```bash
stout pin node@18
```

### Switch Versions

If you have multiple versions installed:

```bash
stout switch python 3.11
```

### Rollback

Revert to a previous version:

```bash
stout rollback python
```

### View History

See version history for a package:

```bash
stout history python
```

---

## Common Workflows

### Developer Setup

```bash
# Install development tools
stout install git node python@3.11 rust

# Install applications
stout cask install visual-studio-code iterm2
```

### Using Brewfiles

If you have an existing Brewfile:

```bash
stout bundle install
```

Create a Brewfile from current installation:

```bash
stout bundle dump
```

### Project-Specific Environments

Create an isolated prefix:

```bash
stout prefix create ~/myproject/.stout
stout --prefix=~/myproject/.stout install node@20 python@3.12
```

---

## Next Steps

- [Command Reference](commands.md) - Complete command documentation
- [Configuration](configuration.md) - Customize stout settings
- [Casks Guide](casks.md) - Working with applications

---

## Coexistence with Homebrew

Stout's `install` writes the same `Cellar/<formula>/<version>/` layout as
Homebrew, links into the same `bin/`, `lib/`, and `share/` directories under
the configured prefix, and emits an `INSTALL_RECEIPT.json` compatible with
the format `brew` reads. This is intentional — you can run both side-by-side
on the same machine and either tool will see the other's packages.

If you adopt stout on a machine that already has Homebrew, the recommended
flow is:

```bash
# 1. Let stout discover what brew installed
stout import

# 2. Verify everything is tracked
stout list --source brew

# 3. From this point, install via stout for the speed wins
stout install <new-package>
```

---

## Source Builds

Bottles (pre-built binaries) are the default. When a bottle is unavailable for
your platform or you explicitly want to build, the source code path is gated
behind `--build-from-source` and supports the same compiler-selection flags
Homebrew uses:

```bash
stout install --build-from-source --jobs=8 --cc=clang --cxx=clang++ neovim
```

`--HEAD` implies `--build-from-source` and fetches the upstream VCS tip
rather than the latest tagged release. Combine with `--keep-bottles` if you
want to retain the downloaded archive after install for debugging.

---

## Resilience Shortcuts

If something goes wrong mid-install, these are the quickest recovery moves
before reaching for [Troubleshooting](troubleshooting.md):

```bash
# Refresh and re-verify the index
stout update --force

# Drop cached downloads and bottles
stout cleanup -s

# Re-create symlinks for a package that disappeared from PATH
stout unlink <pkg> && stout link <pkg>

# Detect and repair drift against the Cellar
stout doctor --fix
```
