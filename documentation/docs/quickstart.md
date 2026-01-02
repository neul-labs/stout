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
