# Configuration

Stout uses a TOML configuration file for customization.

---

## Configuration File

The configuration file is located at:

- **macOS/Linux:** `~/.stout/config.toml`

Stout creates a default configuration on first run.

---

## Full Configuration Reference

```toml
[index]
# Base URL for the package index
base_url = "https://raw.githubusercontent.com/neul-labs/stout-index/main"

# Automatically check for index updates
auto_update = true

# Minimum seconds between auto-updates (default: 30 minutes)
update_interval = 1800

[install]
# Path to the Cellar (where packages are installed)
cellar = "/opt/homebrew/Cellar"

# Path to the prefix (where symlinks are created)
prefix = "/opt/homebrew"

# Number of parallel downloads (1-8)
parallel_downloads = 4

# Default behavior for building from source
build_from_source = false

[cache]
# Maximum cache size (bytes or human-readable)
max_size = "2GB"

# Formula cache TTL in seconds (default: 1 day)
formula_ttl = 86400

# Download cache TTL in seconds (default: 7 days)
download_ttl = 604800

# Cache directory (default: ~/.stout/cache)
# directory = "~/.stout/cache"

[security]
# Require Ed25519 signatures on index updates
require_signature = true

# Allow unsigned packages (not recommended)
allow_unsigned = false

# Maximum age for signatures in seconds (default: 7 days)
max_signature_age = 604800

# Custom trusted public keys (in addition to built-in)
# trusted_keys = ["base64-encoded-public-key"]

[analytics]
# Enable anonymous usage analytics
enabled = false

[output]
# Enable colored output
color = true

# Show progress bars
progress = true

# Verbosity level: "quiet", "normal", "verbose"
verbosity = "normal"
```

---

## Configuration Sections

### Index Settings

Control how Stout fetches package information.

```toml
[index]
base_url = "https://raw.githubusercontent.com/neul-labs/stout-index/main"
auto_update = true
update_interval = 1800
```

| Setting | Description | Default |
|---------|-------------|---------|
| `base_url` | Index repository URL | GitHub stout-index |
| `auto_update` | Check for updates automatically | `true` |
| `update_interval` | Seconds between auto-updates | `1800` (30 min) |

**Private Index:**

For enterprise use, point to your private index:

```toml
[index]
base_url = "https://internal.example.com/stout-index"
```

---

### Installation Settings

Configure package installation behavior.

```toml
[install]
cellar = "/opt/homebrew/Cellar"
prefix = "/opt/homebrew"
parallel_downloads = 4
build_from_source = false
```

| Setting | Description | Default |
|---------|-------------|---------|
| `cellar` | Where packages are installed | `/opt/homebrew/Cellar` |
| `prefix` | Where symlinks are created | `/opt/homebrew` |
| `parallel_downloads` | Concurrent download count | `4` |
| `build_from_source` | Build instead of using bottles | `false` |

**Linux Paths:**

On Linux, the defaults are typically:

```toml
[install]
cellar = "/home/linuxbrew/.linuxbrew/Cellar"
prefix = "/home/linuxbrew/.linuxbrew"
```

---

### Cache Settings

Manage disk usage for cached data.

```toml
[cache]
max_size = "2GB"
formula_ttl = 86400
download_ttl = 604800
```

| Setting | Description | Default |
|---------|-------------|---------|
| `max_size` | Maximum cache size | `2GB` |
| `formula_ttl` | Formula cache lifetime (seconds) | `86400` (1 day) |
| `download_ttl` | Download cache lifetime (seconds) | `604800` (7 days) |

**Size Format:**

Use human-readable sizes: `500MB`, `2GB`, `10GB`

---

### Security Settings

Configure security policies.

```toml
[security]
require_signature = true
allow_unsigned = false
max_signature_age = 604800
```

| Setting | Description | Default |
|---------|-------------|---------|
| `require_signature` | Require Ed25519 signatures | `true` |
| `allow_unsigned` | Allow unsigned packages | `false` |
| `max_signature_age` | Max signature age (seconds) | `604800` (7 days) |

!!! warning "Security Recommendation"
    Keep `require_signature = true` and `allow_unsigned = false` for production use.

**Custom Trusted Keys:**

Add additional trusted public keys:

```toml
[security]
trusted_keys = [
    "your-base64-encoded-ed25519-public-key"
]
```

---

### Output Settings

Control terminal output.

```toml
[output]
color = true
progress = true
verbosity = "normal"
```

| Setting | Description | Default |
|---------|-------------|---------|
| `color` | Colored output | `true` |
| `progress` | Show progress bars | `true` |
| `verbosity` | Output level | `"normal"` |

**Verbosity Levels:**

- `quiet` - Only errors
- `normal` - Standard output
- `verbose` - Detailed output

---

## Environment Variables

Stout respects these environment variables:

| Variable | Description |
|----------|-------------|
| `STOUT_PREFIX` | Override default prefix |
| `STOUT_CELLAR` | Override Cellar location |
| `STOUT_CACHE_DIR` | Override cache directory |
| `STOUT_CONFIG` | Custom config file path |
| `STOUT_INDEX_URL` | Override index URL |
| `STOUT_MIRROR` | Use a mirror for packages |
| `STOUT_NO_COLOR` | Disable colored output |
| `STOUT_VERBOSE` | Enable verbose output |
| `STOUT_QUIET` | Enable quiet mode |

Environment variables override config file settings.

---

## Per-Project Configuration

Create a `.stout.toml` in your project directory for project-specific settings:

```toml
# .stout.toml
[install]
prefix = "./.stout"
cellar = "./.stout/Cellar"
```

Stout checks for `.stout.toml` in the current directory and parent directories.

---

## Viewing Current Configuration

Show the active configuration:

```bash
stout config
```

Show configuration file location:

```bash
stout config --path
```

---

## Example Configurations

### Minimal (Defaults)

```toml
# Empty file uses all defaults
```

### Development Setup

```toml
[output]
verbosity = "verbose"
progress = true

[cache]
max_size = "5GB"
```

### CI/CD Environment

```toml
[output]
color = false
progress = false
verbosity = "quiet"

[analytics]
enabled = false

[index]
auto_update = false
```

### Air-Gapped Environment

```toml
[index]
base_url = "file:///mnt/mirror/stout-index"
auto_update = false

[security]
require_signature = true
```

### Enterprise with Private Index

```toml
[index]
base_url = "https://packages.internal.company.com/stout-index"
update_interval = 3600

[security]
require_signature = true
trusted_keys = [
    "your-company-public-key-base64"
]
```
