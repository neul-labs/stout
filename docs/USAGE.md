# Usage Guide

This guide covers how to use brewx for common package management tasks.

## Basic Commands

### Updating the Index

Before using brewx for the first time, or to get the latest package information:

```bash
brewx update
```

This downloads the latest formula index (~3MB) containing metadata for 8000+ packages.

### Searching for Packages

Search for packages by name or description:

```bash
# Basic search
brewx search json

# Search with limit
brewx search json --limit 10

# Search for exact name
brewx search wget
```

Example output:

```
Found 20 formulas:

  jq              1.7.1   Lightweight and flexible command-line JSON processor
  json-c          0.18    JSON parser for C
  fx              35.0.0  Terminal JSON viewer
  gron            0.7.1   Make JSON greppable
  ...

Use 'brewx info <formula>' for details
```

### Viewing Package Information

Get detailed information about a package:

```bash
brewx info jq
```

Example output:

```
jq 1.7.1
Lightweight and flexible command-line JSON processor

Homepage:    https://jqlang.github.io/jq/
License:     MIT
Tap:         homebrew/core

Dependencies:
  └── oniguruma (runtime)

Bottles:
  ✓ arm64_sonoma  ✓ arm64_ventura  ✓ x86_64_linux  ...

Installed: No
```

### Installing Packages

Install one or more packages:

```bash
# Install a single package
brewx install jq

# Install multiple packages
brewx install jq yq gron

# Install with verbose output
brewx install -v wget
```

brewx will:
1. Resolve all dependencies
2. Download bottles in parallel
3. Extract to Cellar
4. Create symlinks

### Listing Installed Packages

View all installed packages:

```bash
# List all installed packages
brewx list

# List only explicitly installed (not dependencies)
brewx list --requested
```

### Uninstalling Packages

Remove installed packages:

```bash
# Uninstall a package
brewx uninstall jq

# Uninstall multiple packages
brewx uninstall jq yq gron
```

### Upgrading Packages

Upgrade installed packages to latest versions:

```bash
# Upgrade all packages
brewx upgrade

# Upgrade specific packages
brewx upgrade jq wget
```

### System Health Check

Check if your system is properly configured:

```bash
brewx doctor
```

This checks:
- brewx data directory
- Configuration file
- Formula index
- Homebrew prefix and Cellar
- Installed packages state

## Command Reference

### Global Options

These options work with any command:

```bash
-v, --verbose    Enable verbose output
-q, --quiet      Suppress output
-h, --help       Print help
-V, --version    Print version
```

### brewx install

```bash
brewx install [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to install

Options:
  -f, --force          Force reinstall if already installed
  -n, --dry-run        Show what would be installed
  --build-from-source  Build from source instead of using bottles
  -v, --verbose        Enable verbose output
```

### brewx uninstall

```bash
brewx uninstall [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to uninstall

Options:
  -f, --force    Force uninstall even if depended on
  -v, --verbose  Enable verbose output
```

### brewx search

```bash
brewx search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  -l, --limit <N>  Maximum results to show (default: 20)
```

### brewx info

```bash
brewx info <FORMULA>

Arguments:
  <FORMULA>  Formula name
```

### brewx list

```bash
brewx list [OPTIONS]

Options:
  --requested  Show only explicitly installed packages
  --deps       Show only packages installed as dependencies
  --versions   Show version information
```

### brewx update

```bash
brewx update [OPTIONS]

Options:
  -f, --force  Force update even if recently updated
```

### brewx upgrade

```bash
brewx upgrade [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Specific packages to upgrade (default: all)

Options:
  -n, --dry-run  Show what would be upgraded
```

### brewx doctor

```bash
brewx doctor

Checks system health and configuration.
```

### brewx completions

```bash
brewx completions <SHELL>

Arguments:
  <SHELL>  Shell type [bash, zsh, fish, elvish, powershell]
```

## Advanced Usage

### Environment Variables

```bash
# Set log level
RUST_LOG=debug brewx search json

# Override config location
BREWX_CONFIG=/path/to/config.toml brewx update
```

### Using with Existing Homebrew

brewx is designed to coexist with Homebrew:

```bash
# Packages installed by brew are visible to brewx
brew install wget
brewx list  # Shows wget

# Packages installed by brewx are visible to brew
brewx install jq
brew list  # Shows jq
```

### Cache Management

brewx caches downloaded bottles and formula data:

```bash
# Cache location
~/.brewx/cache/downloads/   # Downloaded bottles
~/.brewx/cache/formulas/    # Formula JSON files

# Clear download cache (bottles)
rm -rf ~/.brewx/cache/downloads/*

# Clear formula cache
rm -rf ~/.brewx/cache/formulas/*
```

### Offline Usage

Once you've downloaded the index, many commands work offline:

```bash
# These work offline:
brewx search json
brewx list
brewx doctor

# These require network:
brewx update
brewx info <pkg>  # If not cached
brewx install <pkg>
```

## Examples

### Install a Development Environment

```bash
# Update index first
brewx update

# Install common dev tools
brewx install git gh jq yq ripgrep fd bat eza

# Verify installation
brewx list
```

### Search and Explore

```bash
# Find JSON-related tools
brewx search json

# Find packages with "vim" in name/description
brewx search vim

# Get details on an interesting package
brewx info neovim
```

### Clean Up

```bash
# See what's installed
brewx list

# Remove packages you don't need
brewx uninstall package1 package2

# Clear download cache
rm -rf ~/.brewx/cache/downloads/*
```
