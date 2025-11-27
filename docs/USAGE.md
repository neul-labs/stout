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

### brewx outdated

Show packages with available updates:

```bash
brewx outdated [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Specific packages to check (default: all)

Options:
  -v, --verbose  Show detailed version info
  --json         Output as JSON
  --greedy       Include auto-updated packages
```

### brewx autoremove

Remove packages that were installed as dependencies but are no longer needed:

```bash
brewx autoremove [OPTIONS]

Options:
  -n, --dry-run  Show what would be removed without removing
```

### brewx deps

Show dependencies for a package:

```bash
brewx deps [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to show dependencies for

Options:
  --tree              Show dependencies as a tree
  --graph             Output as DOT graph format (pipe to `dot -Tpng`)
  --json              Output as JSON
  -f, --format        Output format (list, tree, graph, json)
  -a, --all           Include all dependency types
  --installed         Only show installed dependencies
  --include-build     Include build dependencies
  --include-test      Include test dependencies
  --include-optional  Include optional dependencies
  -n, --count         Show only the dependency count
```

Example DOT graph output:

```bash
brewx deps --graph jq | dot -Tpng -o deps.png
```

### brewx uses

Show packages that depend on a given package (reverse dependencies):

```bash
brewx uses [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to find dependents of

Options:
  --recursive   Show recursive dependents
  --installed   Only show installed dependents
```

### brewx why

Show why a package is installed (reverse dependency chain):

```bash
brewx why [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to find the installation reason for

Options:
  --json       Output as JSON
  -a, --all    Show all dependency paths (not just shortest)
```

Example output:

```
jq is installed because:

● ripgrep (requested)
  └─▶ oniguruma depends on
      └─▶ jq
```

### brewx history

Show package version history:

```bash
brewx history [OPTIONS] [FORMULA]

Arguments:
  [FORMULA]  Formula to show history for (omit for all packages)

Options:
  --json       Output as JSON
  -n, --limit  Show only the last N entries
```

### brewx rollback

Rollback a package to a previous version:

```bash
brewx rollback [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to rollback

Options:
  -v, --version  Rollback to a specific version (default: previous)
  -n, --dry-run  Show what would be done
```

### brewx switch

Switch between installed versions of a package:

```bash
brewx switch [OPTIONS] <FORMULA> <VERSION>

Arguments:
  <FORMULA>  Formula to switch
  <VERSION>  Version to switch to

Options:
  -n, --dry-run  Show what would be done
```

### brewx home

Open a package's homepage in your default browser:

```bash
brewx home <FORMULA>

Arguments:
  <FORMULA>  Formula whose homepage to open
```

### brewx pin

Pin a package to prevent it from being upgraded:

```bash
brewx pin <FORMULA>

Arguments:
  <FORMULA>  Formula to pin
```

### brewx unpin

Unpin a package to allow it to be upgraded:

```bash
brewx unpin <FORMULA>

Arguments:
  <FORMULA>  Formula to unpin
```

### brewx link

Create symlinks for a package without reinstalling:

```bash
brewx link [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to link

Options:
  -f, --force      Overwrite existing files
  --overwrite      Synonym for --force
```

### brewx unlink

Remove symlinks for a package without uninstalling:

```bash
brewx unlink <FORMULA>

Arguments:
  <FORMULA>  Formula to unlink
```

### brewx reinstall

Uninstall and reinstall a package:

```bash
brewx reinstall [OPTIONS] <FORMULAS>...

Arguments:
  <FORMULAS>...  Formulas to reinstall

Options:
  -s, --build-from-source  Build from source instead of using bottles
  --keep-bottles           Keep downloaded bottles after installation
```

### brewx cleanup

Remove old downloads and cache files:

```bash
brewx cleanup [OPTIONS] [FORMULAS]...

Arguments:
  [FORMULAS]...  Specific formulas to clean up (default: all)

Options:
  --prune=<DAYS>   Remove downloads older than DAYS (default: 120)
  -s, --scrub      Scrub the cache, including downloads for latest versions
  -n, --dry-run    Show what would be removed without removing
  --prune-prefix   Remove old versions from the Cellar
```

### brewx config

Display brewx configuration and system information:

```bash
brewx config
```

Shows: version, paths, index URL, CPU/OS info, and Rust version.

### brewx services

Manage background services for installed packages:

```bash
brewx services [COMMAND]

Subcommands:
  list              List all managed services (default)
  start <service>   Start a service
  stop <service>    Stop a service
  restart <service> Restart a service
  run <service>     Run a service (foreground, not registered)
  info <service>    Show service information
  cleanup           Clean up unused services
```

### brewx tap

Manage custom formula repositories:

```bash
brewx tap [OPTIONS] [TAP]

Arguments:
  [TAP]  Tap to add (e.g., homebrew/cask)

Options:
  -r, --remove  Remove a tap instead of adding
```

### brewx lock

Manage lockfiles for reproducible environments:

```bash
brewx lock [COMMAND]

Subcommands:
  create            Create a lockfile from currently installed packages
  install           Install packages from a lockfile
  diff              Show differences between lockfile and installed packages
```

## Cask Commands (Applications)

brewx can also manage applications (casks) on macOS and Linux. Casks are packaged macOS applications (DMG, PKG, ZIP) or Linux apps (AppImage, Flatpak).

### brewx cask install

Install applications:

```bash
brewx cask install [OPTIONS] <CASKS>...

Arguments:
  <CASKS>...  Applications to install

Options:
  -f, --force      Force reinstall if already installed
  --no-verify      Skip checksum verification
  --appdir <DIR>   Custom application directory (default: /Applications)
  --dry-run        Show what would be installed
```

Example:

```bash
brewx cask install visual-studio-code firefox slack
```

### brewx cask uninstall

Uninstall applications:

```bash
brewx cask uninstall [OPTIONS] <CASKS>...

Arguments:
  <CASKS>...  Applications to uninstall

Options:
  --zap         Remove preferences and caches (thorough cleanup)
  -f, --force   Force uninstall
```

### brewx cask search

Search for applications:

```bash
brewx cask search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  --json  Output as JSON
```

### brewx cask info

Show application information:

```bash
brewx cask info [OPTIONS] <CASK>

Arguments:
  <CASK>  Application to show info for

Options:
  --format <FORMAT>  Output format (text, json)
```

### brewx cask list

List installed applications:

```bash
brewx cask list [OPTIONS]

Options:
  -v, --versions  Show version information
  --json          Output as JSON
```

### brewx cask outdated

Show applications with available updates:

```bash
brewx cask outdated [OPTIONS]

Options:
  --json  Output as JSON
```

### brewx cask upgrade

Upgrade installed applications:

```bash
brewx cask upgrade [OPTIONS] [CASKS]...

Arguments:
  [CASKS]...  Specific applications to upgrade (default: all)

Options:
  -f, --force    Force upgrade
  --dry-run      Show what would be upgraded
```

### Cask Examples

```bash
# Search for browsers
brewx cask search browser

# Install popular applications
brewx cask install visual-studio-code firefox slack discord

# View application details
brewx cask info visual-studio-code

# List installed applications with versions
brewx cask list --versions

# Check for updates
brewx cask outdated

# Upgrade all applications
brewx cask upgrade

# Upgrade specific applications
brewx cask upgrade firefox slack

# Uninstall with thorough cleanup
brewx cask uninstall --zap discord
```

### Linux Application Support

On Linux, brewx supports AppImage and Flatpak formats:

```bash
# AppImage installation
# Apps are installed to ~/.local/share/brewx/appimages/
# Symlinks are created in ~/.local/bin/

# Flatpak support (if installed)
# Uses user-level flatpak installation

# Desktop entries are created in ~/.local/share/applications/
```

## Bundle Commands (Brewfile)

brewx supports Homebrew's Brewfile format for declaring system packages.

### brewx bundle

Install packages from Brewfile (default subcommand):

```bash
brewx bundle [OPTIONS]

Options:
  -f, --file <FILE>  Path to Brewfile (default: ./Brewfile)
  --dry-run          Show what would be installed
  --force            Force reinstall
  --no-tap           Skip tap entries
  --no-brew          Skip formula entries
  --no-cask          Skip cask entries
```

### brewx bundle dump

Generate Brewfile from installed packages:

```bash
brewx bundle dump [OPTIONS]

Options:
  -f, --file <FILE>  Output file (default: ./Brewfile)
  --force            Overwrite existing file
  --all              Include dependencies (not just requested)
  --stdout           Output to stdout instead of file
```

### brewx bundle check

Check if all Brewfile packages are installed:

```bash
brewx bundle check [OPTIONS]

Options:
  -v, --verbose      Show detailed output
```

### brewx bundle list

List entries in Brewfile:

```bash
brewx bundle list [OPTIONS]

Options:
  --type <TYPE>      Filter by type (tap, brew, cask, mas)
  --json             Output as JSON
```

### brewx bundle cleanup

Remove packages not in Brewfile:

```bash
brewx bundle cleanup [OPTIONS]

Options:
  --dry-run          Show what would be removed
  --force            Force removal
```

### Brewfile Format

brewx supports the standard Homebrew Brewfile format:

```ruby
# Taps
tap "homebrew/cask"
tap "homebrew/cask-fonts"

# Formulas
brew "jq"
brew "ripgrep"
brew "postgresql@15"

# Casks
cask "firefox"
cask "visual-studio-code"

# Mac App Store (requires mas)
mas "Xcode", id: 497799835
```

## Snapshot Commands

Snapshots allow you to save and restore the current state of installed packages.

### brewx snapshot create

Create a new snapshot:

```bash
brewx snapshot create <NAME> [OPTIONS]

Arguments:
  <NAME>  Name for the snapshot

Options:
  -d, --description <DESC>  Optional description
  -f, --force               Overwrite if exists
```

### brewx snapshot list

List all snapshots:

```bash
brewx snapshot list [OPTIONS]

Options:
  --json             Output as JSON
```

### brewx snapshot show

Show snapshot details:

```bash
brewx snapshot show <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  --json             Output as JSON
```

### brewx snapshot restore

Restore a snapshot:

```bash
brewx snapshot restore <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  --dry-run          Show what would be installed
  -f, --force        Force install
```

### brewx snapshot delete

Delete a snapshot:

```bash
brewx snapshot delete <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  -f, --force        Don't ask for confirmation
```

### brewx snapshot export/import

Export and import snapshots:

```bash
# Export to stdout
brewx snapshot export mysetup > backup.json

# Import from stdin
cat backup.json | brewx snapshot import
```

### Snapshot Examples

```bash
# Create a snapshot before major changes
brewx snapshot create before-update --description "Before system update"

# List snapshots
brewx snapshot list

# Preview what would be restored
brewx snapshot restore before-update --dry-run

# Actually restore
brewx snapshot restore before-update

# Export for backup
brewx snapshot export before-update > ~/backups/brewx-snapshot.json
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
~/.brewx/downloads/         # Downloaded bottles
~/.brewx/formulas/          # Formula JSON files

# Clean up old downloads (>120 days) using cleanup command
brewx cleanup

# Clean downloads older than 30 days
brewx cleanup --prune=30

# Remove all cached downloads
brewx cleanup --scrub

# Preview what would be removed
brewx cleanup --dry-run

# Manual cache clearing
rm -rf ~/.brewx/downloads/*
rm -rf ~/.brewx/formulas/*
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

# Check for outdated packages
brewx outdated

# Remove packages you don't need
brewx uninstall package1 package2

# Remove orphaned dependencies
brewx autoremove

# Clean up old downloads (older than 120 days)
brewx cleanup

# Clean all cached downloads
brewx cleanup --scrub
```

### Manage Dependencies

```bash
# Show what a package depends on
brewx deps jq

# Show dependencies as a tree
brewx deps jq --tree

# Show all dependency types (including build/test)
brewx deps jq --all

# Output as DOT graph (for visualization)
brewx deps jq --graph | dot -Tpng -o jq-deps.png

# Output as JSON (for scripting)
brewx deps jq --json

# Find what depends on a package
brewx uses openssl

# Find installed packages that depend on openssl
brewx uses openssl --installed

# Find why a package is installed
brewx why oniguruma
```

### Version History & Rollback

```bash
# View package history
brewx history jq

# View history for all packages
brewx history

# View only the last 5 entries
brewx history jq -n 5

# Rollback to previous version
brewx rollback jq

# Rollback to specific version
brewx rollback jq --version 1.6

# Preview rollback without making changes
brewx rollback jq --dry-run

# Switch between installed versions (if multiple in Cellar)
brewx switch jq 1.7

# Preview switch
brewx switch jq 1.6 --dry-run
```

### Pin Packages

```bash
# Prevent a package from being upgraded
brewx pin postgresql@15

# See pinned packages
brewx list --pinned

# Allow upgrades again
brewx unpin postgresql@15
```

### Service Management

```bash
# List services
brewx services list

# Start a service
brewx services start postgresql

# Stop a service
brewx services stop postgresql

# View service info
brewx services info postgresql
```

### Security Audit

```bash
# Scan all installed packages for vulnerabilities
brewx audit

# Scan specific packages
brewx audit jq openssl

# Update vulnerability database before scanning
brewx audit --update

# Output as JSON
brewx audit --format json

# Only show high/critical severity
brewx audit --severity high

# Fail if vulnerabilities found at severity threshold
brewx audit --fail-on critical
```

## Audit Commands (Vulnerability Scanning)

brewx can scan installed packages for known security vulnerabilities using an offline vulnerability database.

### brewx audit

Scan packages for vulnerabilities:

```bash
brewx audit [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Packages to audit (default: all installed)

Options:
  --update            Update vulnerability database before scanning
  -f, --format <FMT>  Output format: text (default), json
  --severity <SEV>    Minimum severity to report: low, medium, high, critical
  --fail-on <SEV>     Exit with error if vulnerabilities >= severity found
  --show-unmapped     Show packages without vulnerability data
```

### Audit Examples

```bash
# Full audit of all installed packages
brewx audit

# Audit specific packages
brewx audit openssl curl wget

# Update database and audit
brewx audit --update

# CI/CD usage - fail on high+ severity
brewx audit --fail-on high

# Get machine-readable output
brewx audit --format json > vulnerabilities.json

# Only show critical issues
brewx audit --severity critical

# See which packages have no vulnerability data
brewx audit --show-unmapped
```

### Audit Output

Example text output:

```
Auditing 45 packages for vulnerabilities...

CRITICAL CVE-2024-1234 in openssl@3 (3.0.12)
  A critical vulnerability in TLS handshake...
  Fix: 3.0.13
  More info: https://nvd.nist.gov/vuln/detail/CVE-2024-1234

HIGH CVE-2024-5678 in curl (8.4.0)
  Remote code execution via malformed URL...
  Fix: 8.5.0
  More info: https://curl.se/docs/CVE-2024-5678.html

Summary
  1 critical
  1 high
  0 medium
  0 low

  2 total vulnerabilities in 2 packages
```

### How It Works

brewx uses a pre-built vulnerability database that maps Homebrew formulas to known CVEs:

1. **Database sync**: `scripts/sync_vulns.py` fetches vulnerability data from OSV (Open Source Vulnerabilities)
2. **Mapping**: Formulas are mapped to their upstream package ecosystems (npm, PyPI, OSS-Fuzz, etc.)
3. **Version matching**: Installed versions are checked against affected version ranges
4. **Reporting**: Findings are sorted by severity

The vulnerability database is updated periodically and downloaded on first use or with `--update`.

## Mirror Commands (Offline Mode)

brewx supports creating and using offline mirrors for air-gapped environments.

### brewx mirror create

Create a new offline mirror with specified packages:

```bash
brewx mirror create <OUTPUT_DIR> [OPTIONS] <PACKAGES>...

Arguments:
  <OUTPUT_DIR>   Output directory for the mirror
  <PACKAGES>...  Packages to include in the mirror

Options:
  --all-installed        Include all installed packages
  --from-brewfile <FILE> Create from Brewfile
  --cask <CASK>         Include cask (can be repeated)
  --platforms <PLAT>    Target platforms (default: current)
  --all-platforms       Include all platforms (warning: large)
  --no-deps             Skip dependency resolution
  --dry-run             Show what would be downloaded
```

### brewx mirror serve

Serve a mirror via HTTP:

```bash
brewx mirror serve <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  -p, --port <PORT>  Port to listen on (default: 8080)
  --bind <ADDR>      Address to bind to (default: 0.0.0.0)
  --log-access       Enable access logging
```

### brewx mirror info

Show information about a mirror:

```bash
brewx mirror info <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  --json  Output as JSON
```

### brewx mirror verify

Verify mirror integrity:

```bash
brewx mirror verify <PATH> [OPTIONS] [PACKAGES]...

Arguments:
  <PATH>        Path to the mirror directory
  [PACKAGES]... Specific packages to verify (default: all)

Options:
  -v, --verbose  Show verbose output
```

### brewx mirror outdated

Check for outdated packages in mirror:

```bash
brewx mirror outdated <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  --json  Output as JSON
```

### brewx mirror update

Update packages in an existing mirror:

```bash
brewx mirror update <PATH> [OPTIONS] [PACKAGES]...

Arguments:
  <PATH>        Path to the mirror directory
  [PACKAGES]... Packages to update (default: all)

Options:
  --from-brewfile <FILE>  Update from Brewfile
  --dry-run               Show what would be updated
```

### Mirror Examples

```bash
# Create a mirror with specific packages
brewx mirror create ./mirror jq wget curl

# Create a mirror from all installed packages
brewx mirror create ./mirror --all-installed

# Create a mirror for multiple platforms
brewx mirror create ./mirror jq --platforms arm64_sonoma,x86_64_linux

# Serve a mirror on the network
brewx mirror serve ./mirror --port 9000

# Check mirror for outdated packages
brewx mirror outdated ./mirror

# Verify mirror integrity
brewx mirror verify ./mirror --verbose
```

### Using a Mirror

Configure brewx to use a mirror:

```bash
# One-time override
brewx --mirror=http://mirror.internal:8080 install jq

# File-based mirror (USB drive, local mount)
brewx --mirror=file:///mnt/usb/brewx-mirror install jq

# Configure as default in ~/.brewx/config.toml
[mirror]
url = "http://mirror.internal:8080"
fallback = "error"   # error, warn, or silent
verify_checksums = false
```

### Mirror Structure

```
mirror/
├── manifest.json           # Master manifest with checksums
├── formulas/
│   ├── index.db.zst        # Filtered SQLite index
│   ├── data/
│   │   └── <letter>/<name>.json.zst
│   └── bottles/
│       └── <name>-<version>.<platform>.bottle.tar.gz
├── casks/
│   ├── index.db.zst
│   ├── data/
│   └── artifacts/
└── linux-apps/
    ├── index.db.zst
    ├── data/
    └── artifacts/
```

## Developer Tools

brewx includes tools for formula development and package creation.

### brewx bottle

Create and manage binary packages (bottles):

```bash
brewx bottle <SUBCOMMAND>

Subcommands:
  create <PACKAGE>    Create a bottle from an installed package
  info <BOTTLE>       Show information about a bottle file
  verify <BOTTLE>     Verify bottle integrity
```

#### Bottle Examples

```bash
# Create a bottle from an installed package
brewx bottle create jq

# Show bottle metadata
brewx bottle info jq-1.7.1.arm64_linux.bottle.tar.gz

# Verify bottle integrity
brewx bottle verify jq-1.7.1.arm64_linux.bottle.tar.gz
```

### brewx create

Create new formulas or casks from a URL:

```bash
brewx create [OPTIONS] <URL>

Arguments:
  <URL>  URL to the source archive or application

Options:
  --cask              Create a cask instead of a formula
  --name <NAME>       Override the inferred name
  --output <DIR>      Output directory (default: current directory)
```

#### Create Examples

```bash
# Create a formula from a GitHub release
brewx create https://github.com/user/project/archive/v1.0.0.tar.gz

# Create a formula with custom name
brewx create --name myapp https://example.com/source.tar.gz

# Create a cask from a DMG
brewx create --cask https://example.com/App.dmg
```

### brewx test

Run tests on installed packages:

```bash
brewx test [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Packages to test (default: all installed)

Options:
  -v, --verbose  Show detailed test output
```

#### Test Examples

```bash
# Test all installed packages
brewx test

# Test specific packages
brewx test jq wget curl

# Test with verbose output
brewx test jq --verbose
```

### brewx analytics

Manage opt-in anonymous usage analytics:

```bash
brewx analytics <SUBCOMMAND>

Subcommands:
  on      Enable anonymous analytics
  off     Disable analytics (default)
  status  Show current analytics status
  what    Show what data would be collected
```

#### Analytics Examples

```bash
# Check current status
brewx analytics status

# Enable analytics
brewx analytics on

# Disable analytics
brewx analytics off

# See what data is collected
brewx analytics what
```

### Build from Source Options

When installing packages from source, you can customize the build:

```bash
brewx install <PACKAGE> -s [OPTIONS]

Options:
  -s, --build-from-source  Build from source instead of using bottles
  -j, --jobs <N>           Number of parallel build jobs (default: CPU count)
  --cc <COMPILER>          C compiler to use (e.g., clang, gcc)
  --cxx <COMPILER>         C++ compiler to use (e.g., clang++, g++)
```

#### Build Examples

```bash
# Build from source with 8 parallel jobs
brewx install jq -s --jobs=8

# Build with specific compilers
brewx install jq -s --cc=clang --cxx=clang++

# Build with GCC
brewx install openssl -s --cc=gcc --cxx=g++
```

## Multi-Prefix Support

brewx supports multiple installation prefixes for isolated environments.

### brewx prefix

Manage multiple installation prefixes:

```bash
brewx prefix <SUBCOMMAND>

Subcommands:
  create <PATH>    Create a new prefix
  list             List all known prefixes
  info [PATH]      Show prefix information (default: current prefix)
  default <PATH>   Set the default prefix
  remove <PATH>    Remove a prefix
```

### Prefix Create

Create a new isolated prefix:

```bash
brewx prefix create [OPTIONS] <PATH>

Arguments:
  <PATH>  Path for the new prefix

Options:
  -f, --force  Force creation even if directory exists
```

### Prefix Remove

Remove a prefix:

```bash
brewx prefix remove [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to the prefix

Options:
  --packages     Also remove all installed packages
  -f, --force    Force removal without confirmation
```

### Using Custom Prefixes

You can use a custom prefix with any command using the `--prefix` flag:

```bash
# Install to a custom prefix
brewx --prefix=~/project/.brewx install jq python@3.11

# List packages in a custom prefix
brewx --prefix=~/project/.brewx list

# Upgrade packages in a custom prefix
brewx --prefix=~/project/.brewx upgrade
```

### Environment Variable

Set the default prefix via environment variable:

```bash
export BREWX_PREFIX=~/project/.brewx

# All commands now use the custom prefix
brewx install jq    # Installs to ~/project/.brewx
brewx list          # Lists packages in ~/project/.brewx
```

### Prefix Examples

```bash
# Create an isolated prefix for a project
brewx prefix create ~/projects/myapp/.brewx

# View prefix information
brewx prefix info ~/projects/myapp/.brewx

# Install packages to the project prefix
brewx --prefix=~/projects/myapp/.brewx install python@3.11 node@20

# List all known prefixes
brewx prefix list

# Set as default prefix
brewx prefix default ~/projects/myapp/.brewx

# Add to PATH for project
export PATH="$HOME/projects/myapp/.brewx/bin:$PATH"

# Remove prefix when no longer needed
brewx prefix remove ~/projects/myapp/.brewx --packages --force
```

### Prefix Structure

When you create a prefix, the following directory structure is created:

```
~/project/.brewx/
├── .brewx-prefix       # Marker file
├── Cellar/             # Installed package versions
├── bin/                # Executable symlinks
├── lib/                # Library symlinks
├── include/            # Header symlinks
├── share/              # Shared data symlinks
├── etc/                # Configuration files
└── var/                # Variable data
```

### Use Cases for Multi-Prefix

1. **Project-specific dependencies**: Isolate dependencies per project
2. **Version testing**: Test different package versions without affecting system
3. **CI/CD environments**: Create reproducible build environments
4. **Development sandboxes**: Experiment without risk to main installation
