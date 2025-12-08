# Usage Guide

This guide covers how to use stout for common package management tasks.

## Basic Commands

### Updating the Index

Before using stout for the first time, or to get the latest package information:

```bash
stout update
```

This downloads the latest formula index (~3MB) containing metadata for 8000+ packages.

### Searching for Packages

Search for packages by name or description:

```bash
# Basic search
stout search json

# Search with limit
stout search json --limit 10

# Search for exact name
stout search wget
```

Example output:

```
Found 20 formulas:

  jq              1.7.1   Lightweight and flexible command-line JSON processor
  json-c          0.18    JSON parser for C
  fx              35.0.0  Terminal JSON viewer
  gron            0.7.1   Make JSON greppable
  ...

Use 'stout info <formula>' for details
```

### Viewing Package Information

Get detailed information about a package:

```bash
stout info jq
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
stout install jq

# Install multiple packages
stout install jq yq gron

# Install with verbose output
stout install -v wget
```

stout will:
1. Resolve all dependencies
2. Download bottles in parallel
3. Extract to Cellar
4. Create symlinks

### Listing Installed Packages

View all installed packages:

```bash
# List all installed packages
stout list

# List only explicitly installed (not dependencies)
stout list --requested
```

### Uninstalling Packages

Remove installed packages:

```bash
# Uninstall a package
stout uninstall jq

# Uninstall multiple packages
stout uninstall jq yq gron
```

### Upgrading Packages

Upgrade installed packages to latest versions:

```bash
# Upgrade all packages
stout upgrade

# Upgrade specific packages
stout upgrade jq wget
```

### System Health Check

Check if your system is properly configured:

```bash
stout doctor
```

This checks:
- stout data directory
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

### stout install

```bash
stout install [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to install

Options:
  -f, --force          Force reinstall if already installed
  -n, --dry-run        Show what would be installed
  --build-from-source  Build from source instead of using bottles
  -v, --verbose        Enable verbose output
```

### stout uninstall

```bash
stout uninstall [OPTIONS] <PACKAGES>...

Arguments:
  <PACKAGES>...  Packages to uninstall

Options:
  -f, --force    Force uninstall even if depended on
  -v, --verbose  Enable verbose output
```

### stout search

```bash
stout search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  -l, --limit <N>  Maximum results to show (default: 20)
```

### stout info

```bash
stout info <FORMULA>

Arguments:
  <FORMULA>  Formula name
```

### stout list

```bash
stout list [OPTIONS]

Options:
  --requested  Show only explicitly installed packages
  --deps       Show only packages installed as dependencies
  --versions   Show version information
```

### stout update

```bash
stout update [OPTIONS]

Options:
  -f, --force  Force update even if recently updated
```

### stout upgrade

```bash
stout upgrade [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Specific packages to upgrade (default: all)

Options:
  -n, --dry-run  Show what would be upgraded
```

### stout doctor

```bash
stout doctor

Checks system health and configuration.
```

### stout completions

```bash
stout completions <SHELL>

Arguments:
  <SHELL>  Shell type [bash, zsh, fish, elvish, powershell]
```

### stout outdated

Show packages with available updates:

```bash
stout outdated [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Specific packages to check (default: all)

Options:
  -v, --verbose  Show detailed version info
  --json         Output as JSON
  --greedy       Include auto-updated packages
```

### stout autoremove

Remove packages that were installed as dependencies but are no longer needed:

```bash
stout autoremove [OPTIONS]

Options:
  -n, --dry-run  Show what would be removed without removing
```

### stout deps

Show dependencies for a package:

```bash
stout deps [OPTIONS] <FORMULA>

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
stout deps --graph jq | dot -Tpng -o deps.png
```

### stout uses

Show packages that depend on a given package (reverse dependencies):

```bash
stout uses [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to find dependents of

Options:
  --recursive   Show recursive dependents
  --installed   Only show installed dependents
```

### stout why

Show why a package is installed (reverse dependency chain):

```bash
stout why [OPTIONS] <FORMULA>

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

### stout history

Show package version history:

```bash
stout history [OPTIONS] [FORMULA]

Arguments:
  [FORMULA]  Formula to show history for (omit for all packages)

Options:
  --json       Output as JSON
  -n, --limit  Show only the last N entries
```

### stout rollback

Rollback a package to a previous version:

```bash
stout rollback [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to rollback

Options:
  -v, --version  Rollback to a specific version (default: previous)
  -n, --dry-run  Show what would be done
```

### stout switch

Switch between installed versions of a package:

```bash
stout switch [OPTIONS] <FORMULA> <VERSION>

Arguments:
  <FORMULA>  Formula to switch
  <VERSION>  Version to switch to

Options:
  -n, --dry-run  Show what would be done
```

### stout home

Open a package's homepage in your default browser:

```bash
stout home <FORMULA>

Arguments:
  <FORMULA>  Formula whose homepage to open
```

### stout pin

Pin a package to prevent it from being upgraded:

```bash
stout pin <FORMULA>

Arguments:
  <FORMULA>  Formula to pin
```

### stout unpin

Unpin a package to allow it to be upgraded:

```bash
stout unpin <FORMULA>

Arguments:
  <FORMULA>  Formula to unpin
```

### stout link

Create symlinks for a package without reinstalling:

```bash
stout link [OPTIONS] <FORMULA>

Arguments:
  <FORMULA>  Formula to link

Options:
  -f, --force      Overwrite existing files
  --overwrite      Synonym for --force
```

### stout unlink

Remove symlinks for a package without uninstalling:

```bash
stout unlink <FORMULA>

Arguments:
  <FORMULA>  Formula to unlink
```

### stout reinstall

Uninstall and reinstall a package:

```bash
stout reinstall [OPTIONS] <FORMULAS>...

Arguments:
  <FORMULAS>...  Formulas to reinstall

Options:
  -s, --build-from-source  Build from source instead of using bottles
  --keep-bottles           Keep downloaded bottles after installation
```

### stout cleanup

Remove old downloads and cache files:

```bash
stout cleanup [OPTIONS] [FORMULAS]...

Arguments:
  [FORMULAS]...  Specific formulas to clean up (default: all)

Options:
  --prune=<DAYS>   Remove downloads older than DAYS (default: 120)
  -s, --scrub      Scrub the cache, including downloads for latest versions
  -n, --dry-run    Show what would be removed without removing
  --prune-prefix   Remove old versions from the Cellar
```

### stout config

Display stout configuration and system information:

```bash
stout config
```

Shows: version, paths, index URL, CPU/OS info, and Rust version.

### stout services

Manage background services for installed packages:

```bash
stout services [COMMAND]

Subcommands:
  list              List all managed services (default)
  start <service>   Start a service
  stop <service>    Stop a service
  restart <service> Restart a service
  run <service>     Run a service (foreground, not registered)
  info <service>    Show service information
  cleanup           Clean up unused services
```

### stout tap

Manage custom formula repositories:

```bash
stout tap [OPTIONS] [TAP]

Arguments:
  [TAP]  Tap to add (e.g., homebrew/cask)

Options:
  -r, --remove  Remove a tap instead of adding
```

### stout lock

Manage lockfiles for reproducible environments:

```bash
stout lock [COMMAND]

Subcommands:
  create            Create a lockfile from currently installed packages
  install           Install packages from a lockfile
  diff              Show differences between lockfile and installed packages
```

## Cask Commands (Applications)

stout can also manage applications (casks) on macOS and Linux. Casks are packaged macOS applications (DMG, PKG, ZIP) or Linux apps (AppImage, Flatpak).

### stout cask install

Install applications:

```bash
stout cask install [OPTIONS] <CASKS>...

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
stout cask install visual-studio-code firefox slack
```

### stout cask uninstall

Uninstall applications:

```bash
stout cask uninstall [OPTIONS] <CASKS>...

Arguments:
  <CASKS>...  Applications to uninstall

Options:
  --zap         Remove preferences and caches (thorough cleanup)
  -f, --force   Force uninstall
```

### stout cask search

Search for applications:

```bash
stout cask search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
  --json  Output as JSON
```

### stout cask info

Show application information:

```bash
stout cask info [OPTIONS] <CASK>

Arguments:
  <CASK>  Application to show info for

Options:
  --format <FORMAT>  Output format (text, json)
```

### stout cask list

List installed applications:

```bash
stout cask list [OPTIONS]

Options:
  -v, --versions  Show version information
  --json          Output as JSON
```

### stout cask outdated

Show applications with available updates:

```bash
stout cask outdated [OPTIONS]

Options:
  --json  Output as JSON
```

### stout cask upgrade

Upgrade installed applications:

```bash
stout cask upgrade [OPTIONS] [CASKS]...

Arguments:
  [CASKS]...  Specific applications to upgrade (default: all)

Options:
  -f, --force    Force upgrade
  --dry-run      Show what would be upgraded
```

### Cask Examples

```bash
# Search for browsers
stout cask search browser

# Install popular applications
stout cask install visual-studio-code firefox slack discord

# View application details
stout cask info visual-studio-code

# List installed applications with versions
stout cask list --versions

# Check for updates
stout cask outdated

# Upgrade all applications
stout cask upgrade

# Upgrade specific applications
stout cask upgrade firefox slack

# Uninstall with thorough cleanup
stout cask uninstall --zap discord
```

### Linux Application Support

On Linux, stout supports AppImage and Flatpak formats:

```bash
# AppImage installation
# Apps are installed to ~/.local/share/stout/appimages/
# Symlinks are created in ~/.local/bin/

# Flatpak support (if installed)
# Uses user-level flatpak installation

# Desktop entries are created in ~/.local/share/applications/
```

## Bundle Commands (Brewfile)

stout supports Homebrew's Brewfile format for declaring system packages.

### stout bundle

Install packages from Brewfile (default subcommand):

```bash
stout bundle [OPTIONS]

Options:
  -f, --file <FILE>  Path to Brewfile (default: ./Brewfile)
  --dry-run          Show what would be installed
  --force            Force reinstall
  --no-tap           Skip tap entries
  --no-brew          Skip formula entries
  --no-cask          Skip cask entries
```

### stout bundle dump

Generate Brewfile from installed packages:

```bash
stout bundle dump [OPTIONS]

Options:
  -f, --file <FILE>  Output file (default: ./Brewfile)
  --force            Overwrite existing file
  --all              Include dependencies (not just requested)
  --stdout           Output to stdout instead of file
```

### stout bundle check

Check if all Brewfile packages are installed:

```bash
stout bundle check [OPTIONS]

Options:
  -v, --verbose      Show detailed output
```

### stout bundle list

List entries in Brewfile:

```bash
stout bundle list [OPTIONS]

Options:
  --type <TYPE>      Filter by type (tap, brew, cask, mas)
  --json             Output as JSON
```

### stout bundle cleanup

Remove packages not in Brewfile:

```bash
stout bundle cleanup [OPTIONS]

Options:
  --dry-run          Show what would be removed
  --force            Force removal
```

### Brewfile Format

stout supports the standard Homebrew Brewfile format:

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

### stout snapshot create

Create a new snapshot:

```bash
stout snapshot create <NAME> [OPTIONS]

Arguments:
  <NAME>  Name for the snapshot

Options:
  -d, --description <DESC>  Optional description
  -f, --force               Overwrite if exists
```

### stout snapshot list

List all snapshots:

```bash
stout snapshot list [OPTIONS]

Options:
  --json             Output as JSON
```

### stout snapshot show

Show snapshot details:

```bash
stout snapshot show <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  --json             Output as JSON
```

### stout snapshot restore

Restore a snapshot:

```bash
stout snapshot restore <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  --dry-run          Show what would be installed
  -f, --force        Force install
```

### stout snapshot delete

Delete a snapshot:

```bash
stout snapshot delete <NAME> [OPTIONS]

Arguments:
  <NAME>  Snapshot name

Options:
  -f, --force        Don't ask for confirmation
```

### stout snapshot export/import

Export and import snapshots:

```bash
# Export to stdout
stout snapshot export mysetup > backup.json

# Import from stdin
cat backup.json | stout snapshot import
```

### Snapshot Examples

```bash
# Create a snapshot before major changes
stout snapshot create before-update --description "Before system update"

# List snapshots
stout snapshot list

# Preview what would be restored
stout snapshot restore before-update --dry-run

# Actually restore
stout snapshot restore before-update

# Export for backup
stout snapshot export before-update > ~/backups/stout-snapshot.json
```

## Advanced Usage

### Environment Variables

```bash
# Set log level
RUST_LOG=debug stout search json

# Override config location
STOUT_CONFIG=/path/to/config.toml stout update
```

### Using with Existing Homebrew

stout is designed to coexist with Homebrew:

```bash
# Packages installed by brew are visible to stout
brew install wget
stout list  # Shows wget

# Packages installed by stout are visible to brew
stout install jq
brew list  # Shows jq
```

### Cache Management

stout caches downloaded bottles and formula data:

```bash
# Cache location
~/.stout/downloads/         # Downloaded bottles
~/.stout/formulas/          # Formula JSON files

# Clean up old downloads (>120 days) using cleanup command
stout cleanup

# Clean downloads older than 30 days
stout cleanup --prune=30

# Remove all cached downloads
stout cleanup --scrub

# Preview what would be removed
stout cleanup --dry-run

# Manual cache clearing
rm -rf ~/.stout/downloads/*
rm -rf ~/.stout/formulas/*
```

### Offline Usage

Once you've downloaded the index, many commands work offline:

```bash
# These work offline:
stout search json
stout list
stout doctor

# These require network:
stout update
stout info <pkg>  # If not cached
stout install <pkg>
```

## Examples

### Install a Development Environment

```bash
# Update index first
stout update

# Install common dev tools
stout install git gh jq yq ripgrep fd bat eza

# Verify installation
stout list
```

### Search and Explore

```bash
# Find JSON-related tools
stout search json

# Find packages with "vim" in name/description
stout search vim

# Get details on an interesting package
stout info neovim
```

### Clean Up

```bash
# See what's installed
stout list

# Check for outdated packages
stout outdated

# Remove packages you don't need
stout uninstall package1 package2

# Remove orphaned dependencies
stout autoremove

# Clean up old downloads (older than 120 days)
stout cleanup

# Clean all cached downloads
stout cleanup --scrub
```

### Manage Dependencies

```bash
# Show what a package depends on
stout deps jq

# Show dependencies as a tree
stout deps jq --tree

# Show all dependency types (including build/test)
stout deps jq --all

# Output as DOT graph (for visualization)
stout deps jq --graph | dot -Tpng -o jq-deps.png

# Output as JSON (for scripting)
stout deps jq --json

# Find what depends on a package
stout uses openssl

# Find installed packages that depend on openssl
stout uses openssl --installed

# Find why a package is installed
stout why oniguruma
```

### Version History & Rollback

```bash
# View package history
stout history jq

# View history for all packages
stout history

# View only the last 5 entries
stout history jq -n 5

# Rollback to previous version
stout rollback jq

# Rollback to specific version
stout rollback jq --version 1.6

# Preview rollback without making changes
stout rollback jq --dry-run

# Switch between installed versions (if multiple in Cellar)
stout switch jq 1.7

# Preview switch
stout switch jq 1.6 --dry-run
```

### Pin Packages

```bash
# Prevent a package from being upgraded
stout pin postgresql@15

# See pinned packages
stout list --pinned

# Allow upgrades again
stout unpin postgresql@15
```

### Service Management

```bash
# List services
stout services list

# Start a service
stout services start postgresql

# Stop a service
stout services stop postgresql

# View service info
stout services info postgresql
```

### Security Audit

```bash
# Scan all installed packages for vulnerabilities
stout audit

# Scan specific packages
stout audit jq openssl

# Update vulnerability database before scanning
stout audit --update

# Output as JSON
stout audit --format json

# Only show high/critical severity
stout audit --severity high

# Fail if vulnerabilities found at severity threshold
stout audit --fail-on critical
```

## Audit Commands (Vulnerability Scanning)

stout can scan installed packages for known security vulnerabilities using an offline vulnerability database.

### stout audit

Scan packages for vulnerabilities:

```bash
stout audit [OPTIONS] [PACKAGES]...

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
stout audit

# Audit specific packages
stout audit openssl curl wget

# Update database and audit
stout audit --update

# CI/CD usage - fail on high+ severity
stout audit --fail-on high

# Get machine-readable output
stout audit --format json > vulnerabilities.json

# Only show critical issues
stout audit --severity critical

# See which packages have no vulnerability data
stout audit --show-unmapped
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

stout uses a pre-built vulnerability database that maps Homebrew formulas to known CVEs:

1. **Database sync**: `scripts/sync_vulns.py` fetches vulnerability data from OSV (Open Source Vulnerabilities)
2. **Mapping**: Formulas are mapped to their upstream package ecosystems (npm, PyPI, OSS-Fuzz, etc.)
3. **Version matching**: Installed versions are checked against affected version ranges
4. **Reporting**: Findings are sorted by severity

The vulnerability database is updated periodically and downloaded on first use or with `--update`.

## Mirror Commands (Offline Mode)

stout supports creating and using offline mirrors for air-gapped environments.

### stout mirror create

Create a new offline mirror with specified packages:

```bash
stout mirror create <OUTPUT_DIR> [OPTIONS] <PACKAGES>...

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

### stout mirror serve

Serve a mirror via HTTP:

```bash
stout mirror serve <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  -p, --port <PORT>  Port to listen on (default: 8080)
  --bind <ADDR>      Address to bind to (default: 0.0.0.0)
  --log-access       Enable access logging
```

### stout mirror info

Show information about a mirror:

```bash
stout mirror info <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  --json  Output as JSON
```

### stout mirror verify

Verify mirror integrity:

```bash
stout mirror verify <PATH> [OPTIONS] [PACKAGES]...

Arguments:
  <PATH>        Path to the mirror directory
  [PACKAGES]... Specific packages to verify (default: all)

Options:
  -v, --verbose  Show verbose output
```

### stout mirror outdated

Check for outdated packages in mirror:

```bash
stout mirror outdated <PATH> [OPTIONS]

Arguments:
  <PATH>  Path to the mirror directory

Options:
  --json  Output as JSON
```

### stout mirror update

Update packages in an existing mirror:

```bash
stout mirror update <PATH> [OPTIONS] [PACKAGES]...

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
stout mirror create ./mirror jq wget curl

# Create a mirror from all installed packages
stout mirror create ./mirror --all-installed

# Create a mirror for multiple platforms
stout mirror create ./mirror jq --platforms arm64_sonoma,x86_64_linux

# Serve a mirror on the network
stout mirror serve ./mirror --port 9000

# Check mirror for outdated packages
stout mirror outdated ./mirror

# Verify mirror integrity
stout mirror verify ./mirror --verbose
```

### Using a Mirror

Configure stout to use a mirror:

```bash
# One-time override
stout --mirror=http://mirror.internal:8080 install jq

# File-based mirror (USB drive, local mount)
stout --mirror=file:///mnt/usb/stout-mirror install jq

# Configure as default in ~/.stout/config.toml
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

stout includes tools for formula development and package creation.

### stout bottle

Create and manage binary packages (bottles):

```bash
stout bottle <SUBCOMMAND>

Subcommands:
  create <PACKAGE>    Create a bottle from an installed package
  info <BOTTLE>       Show information about a bottle file
  verify <BOTTLE>     Verify bottle integrity
```

#### Bottle Examples

```bash
# Create a bottle from an installed package
stout bottle create jq

# Show bottle metadata
stout bottle info jq-1.7.1.arm64_linux.bottle.tar.gz

# Verify bottle integrity
stout bottle verify jq-1.7.1.arm64_linux.bottle.tar.gz
```

### stout create

Create new formulas or casks from a URL:

```bash
stout create [OPTIONS] <URL>

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
stout create https://github.com/user/project/archive/v1.0.0.tar.gz

# Create a formula with custom name
stout create --name myapp https://example.com/source.tar.gz

# Create a cask from a DMG
stout create --cask https://example.com/App.dmg
```

### stout test

Run tests on installed packages:

```bash
stout test [OPTIONS] [PACKAGES]...

Arguments:
  [PACKAGES]...  Packages to test (default: all installed)

Options:
  -v, --verbose  Show detailed test output
```

#### Test Examples

```bash
# Test all installed packages
stout test

# Test specific packages
stout test jq wget curl

# Test with verbose output
stout test jq --verbose
```

### stout analytics

Manage opt-in anonymous usage analytics:

```bash
stout analytics <SUBCOMMAND>

Subcommands:
  on      Enable anonymous analytics
  off     Disable analytics (default)
  status  Show current analytics status
  what    Show what data would be collected
```

#### Analytics Examples

```bash
# Check current status
stout analytics status

# Enable analytics
stout analytics on

# Disable analytics
stout analytics off

# See what data is collected
stout analytics what
```

### Build from Source Options

When installing packages from source, you can customize the build:

```bash
stout install <PACKAGE> -s [OPTIONS]

Options:
  -s, --build-from-source  Build from source instead of using bottles
  -j, --jobs <N>           Number of parallel build jobs (default: CPU count)
  --cc <COMPILER>          C compiler to use (e.g., clang, gcc)
  --cxx <COMPILER>         C++ compiler to use (e.g., clang++, g++)
```

#### Build Examples

```bash
# Build from source with 8 parallel jobs
stout install jq -s --jobs=8

# Build with specific compilers
stout install jq -s --cc=clang --cxx=clang++

# Build with GCC
stout install openssl -s --cc=gcc --cxx=g++
```

## Multi-Prefix Support

stout supports multiple installation prefixes for isolated environments.

### stout prefix

Manage multiple installation prefixes:

```bash
stout prefix <SUBCOMMAND>

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
stout prefix create [OPTIONS] <PATH>

Arguments:
  <PATH>  Path for the new prefix

Options:
  -f, --force  Force creation even if directory exists
```

### Prefix Remove

Remove a prefix:

```bash
stout prefix remove [OPTIONS] <PATH>

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
stout --prefix=~/project/.stout install jq python@3.11

# List packages in a custom prefix
stout --prefix=~/project/.stout list

# Upgrade packages in a custom prefix
stout --prefix=~/project/.stout upgrade
```

### Environment Variable

Set the default prefix via environment variable:

```bash
export STOUT_PREFIX=~/project/.stout

# All commands now use the custom prefix
stout install jq    # Installs to ~/project/.stout
stout list          # Lists packages in ~/project/.stout
```

### Prefix Examples

```bash
# Create an isolated prefix for a project
stout prefix create ~/projects/myapp/.stout

# View prefix information
stout prefix info ~/projects/myapp/.stout

# Install packages to the project prefix
stout --prefix=~/projects/myapp/.stout install python@3.11 node@20

# List all known prefixes
stout prefix list

# Set as default prefix
stout prefix default ~/projects/myapp/.stout

# Add to PATH for project
export PATH="$HOME/projects/myapp/.stout/bin:$PATH"

# Remove prefix when no longer needed
stout prefix remove ~/projects/myapp/.stout --packages --force
```

### Prefix Structure

When you create a prefix, the following directory structure is created:

```
~/project/.stout/
├── .stout-prefix       # Marker file
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
