# Command Reference

Complete reference for all Stout commands.

---

## Package Management

### install

Install one or more packages.

```bash
stout install <package>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `--build-from-source` | Compile from source instead of using bottles |
| `--force` | Install even if already installed |
| `--ignore-dependencies` | Skip dependency installation |
| `--only-dependencies` | Install only dependencies, not the package |
| `--cask` | Install as a cask (application) |
| `--quiet` | Suppress output |

**Examples:**

```bash
# Install a package
stout install jq

# Install multiple packages
stout install jq yq gron

# Install a specific version
stout install python@3.11

# Build from source
stout install --build-from-source neovim
```

---

### uninstall

Remove installed packages.

```bash
stout uninstall <package>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `--force` | Delete all installed versions |
| `--ignore-dependencies` | Don't fail if dependents exist |
| `--zap` | Remove all files (casks only) |

**Examples:**

```bash
stout uninstall jq
stout uninstall --force python
```

---

### reinstall

Reinstall a package.

```bash
stout reinstall <package>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `--build-from-source` | Compile from source |
| `--force` | Reinstall even if up to date |

---

### upgrade

Upgrade installed packages.

```bash
stout upgrade [package]...
```

Without arguments, upgrades all outdated packages.

**Options:**

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be upgraded |
| `--fetch-HEAD` | Fetch upstream repository for HEAD installs |
| `--greedy` | Upgrade casks with auto-updates |

**Examples:**

```bash
# Upgrade specific package
stout upgrade python

# Upgrade all packages
stout upgrade

# Preview upgrades
stout upgrade --dry-run
```

---

### update

Update the formula index.

```bash
stout update
```

Downloads the latest package index from the stout-index repository.

**Options:**

| Option | Description |
|--------|-------------|
| `--force` | Force update even if recently updated |

---

### outdated

List packages with available updates.

```bash
stout outdated
```

**Options:**

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |
| `--quiet` | Only show package names |
| `--cask` | Check casks instead of formulas |

---

### autoremove

Remove packages installed as dependencies that are no longer needed.

```bash
stout autoremove
```

**Options:**

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview what would be removed |

---

## Discovery & Information

### search

Search for packages.

```bash
stout search <query>
```

Uses FTS5 full-text search for fast results.

**Options:**

| Option | Description |
|--------|-------------|
| `--desc` | Search in descriptions |
| `--cask` | Search casks instead of formulas |
| `--json` | Output as JSON |

**Examples:**

```bash
stout search json
stout search --desc "image processing"
stout search --cask browser
```

---

### info

Show package information.

```bash
stout info <package>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--json` | Output as JSON |
| `--installed` | Only show installed packages |
| `--cask` | Show cask info |

**Examples:**

```bash
stout info jq
stout info --json python@3.11
```

---

### list

List installed packages.

```bash
stout list
```

**Options:**

| Option | Description |
|--------|-------------|
| `--versions` | Show installed versions |
| `--pinned` | Only show pinned packages |
| `--cask` | List installed casks |
| `--json` | Output as JSON |
| `-1` | One package per line |

---

### deps

Show package dependencies.

```bash
stout deps <package>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--tree` | Show as dependency tree |
| `--graph` | Output DOT graph format |
| `--installed` | Only show installed dependencies |
| `--all` | Show all dependencies recursively |
| `--json` | Output as JSON |

**Examples:**

```bash
stout deps python@3.11
stout deps --tree ffmpeg
stout deps --graph imagemagick > deps.dot
```

---

### uses

Show packages that depend on a package (reverse dependencies).

```bash
stout uses <package>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--recursive` | Show indirect dependents |
| `--installed` | Only show installed dependents |

---

### why

Explain why a package is installed.

```bash
stout why <package>
```

Shows the dependency chain that led to installation.

---

### home

Open package homepage in browser.

```bash
stout home <package>
```

---

## Package Control

### pin

Prevent a package from being upgraded.

```bash
stout pin <package>...
```

---

### unpin

Allow a pinned package to be upgraded.

```bash
stout unpin <package>...
```

---

### link

Create symlinks for a package.

```bash
stout link <package>
```

**Options:**

| Option | Description |
|--------|-------------|
| `--overwrite` | Overwrite existing symlinks |
| `--force` | Allow linking keg-only packages |
| `--dry-run` | Preview what would be linked |

---

### unlink

Remove symlinks for a package.

```bash
stout unlink <package>
```

The package remains installed but is not in PATH.

---

### switch

Switch between installed versions.

```bash
stout switch <package> <version>
```

**Example:**

```bash
stout switch python 3.11
```

---

### rollback

Rollback to a previous version.

```bash
stout rollback <package>
```

Reverts to the previously installed version.

---

## System & Maintenance

### cleanup

Remove old versions and cached downloads.

```bash
stout cleanup
```

**Options:**

| Option | Description |
|--------|-------------|
| `--dry-run` | Preview what would be removed |
| `--prune=<days>` | Remove downloads older than days |
| `-s` | Scrub the cache |

---

### doctor

Check system for potential problems.

```bash
stout doctor
```

Runs diagnostics and reports issues.

---

### config

Show or modify configuration.

```bash
stout config
```

Displays current configuration settings.

---

### services

Manage background services.

```bash
stout services <command> [service]
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `list` | List all services |
| `start <service>` | Start a service |
| `stop <service>` | Stop a service |
| `restart <service>` | Restart a service |
| `run <service>` | Run in foreground |

---

### tap

Manage third-party repositories.

```bash
stout tap [user/repo]
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `stout tap` | List tapped repositories |
| `stout tap <user/repo>` | Add a tap |
| `stout untap <user/repo>` | Remove a tap |

---

### history

Show version history for a package.

```bash
stout history <package>
```

---

### completions

Generate shell completions.

```bash
stout completions <shell>
```

**Shells:** `bash`, `zsh`, `fish`

---

## Cask Commands

Casks are macOS applications and Linux apps.

### cask install

```bash
stout cask install <cask>...
```

### cask uninstall

```bash
stout cask uninstall <cask>...
```

**Options:**

| Option | Description |
|--------|-------------|
| `--zap` | Remove all associated files |

### cask search

```bash
stout cask search <query>
```

### cask info

```bash
stout cask info <cask>
```

### cask list

```bash
stout cask list
```

### cask outdated

```bash
stout cask outdated
```

### cask upgrade

```bash
stout cask upgrade [cask]...
```

---

## Bundle & Snapshot

### bundle

Work with Brewfiles.

```bash
stout bundle <command>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `install` | Install from Brewfile |
| `dump` | Create Brewfile from installed packages |
| `check` | Check if Brewfile is satisfied |
| `list` | List packages from Brewfile |
| `cleanup` | Remove packages not in Brewfile |

**Examples:**

```bash
# Install from Brewfile
stout bundle install

# Create Brewfile
stout bundle dump > Brewfile

# Check status
stout bundle check
```

---

### snapshot

Save and restore system state.

```bash
stout snapshot <command>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `save <name>` | Save current state |
| `restore <name>` | Restore a snapshot |
| `list` | List snapshots |
| `delete <name>` | Delete a snapshot |

---

## Security & Audit

### audit

Scan for known vulnerabilities.

```bash
stout audit [package]...
```

Without arguments, scans all installed packages.

**Options:**

| Option | Description |
|--------|-------------|
| `--severity=<level>` | Minimum severity (low, medium, high, critical) |
| `--json` | Output as JSON |

---

## Offline & Mirroring

### mirror

Manage offline mirrors.

```bash
stout mirror <command>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `create <path> <package>...` | Create a mirror with packages |
| `serve <path>` | Serve mirror via HTTP |
| `info <path>` | Show mirror information |
| `verify <path>` | Verify mirror integrity |

**Examples:**

```bash
# Create mirror with essential tools
stout mirror create ./mirror jq curl wget git

# Serve on local network
stout mirror serve ./mirror --port 9000

# Use mirror
stout --mirror=http://localhost:9000 install jq
```

---

## Multi-Prefix

### prefix

Manage isolated environments.

```bash
stout prefix <command>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `create <path>` | Create new prefix |
| `list` | List prefixes |
| `info <path>` | Show prefix information |
| `default <path>` | Set default prefix |
| `remove <path>` | Remove a prefix |

**Examples:**

```bash
# Create project-specific environment
stout prefix create ~/project/.stout

# Install packages to prefix
stout --prefix=~/project/.stout install node@20

# Set as default
stout prefix default ~/project/.stout
```

---

## Developer Commands

### bottle

Create binary packages.

```bash
stout bottle <package>
```

Creates a distributable bottle from an installed package.

---

### create

Create a formula from a URL.

```bash
stout create <url>
```

Generates a formula template from a source archive.

---

### test

Run package tests.

```bash
stout test <package>
```

Runs the test block defined in the formula.

---

### analytics

Manage usage analytics.

```bash
stout analytics <command>
```

**Subcommands:**

| Command | Description |
|---------|-------------|
| `state` | Show analytics status |
| `on` | Enable analytics |
| `off` | Disable analytics |

---

## Global Options

These options work with most commands:

| Option | Description |
|--------|-------------|
| `--prefix=<path>` | Use custom prefix |
| `--mirror=<url>` | Use custom mirror |
| `--verbose` | Show detailed output |
| `--quiet` | Suppress output |
| `--json` | Output as JSON (where supported) |
| `--help` | Show help for command |
| `--version` | Show version |
