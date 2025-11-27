# Enterprise Deployment Guide

brewx is designed for enterprise environments with features for private hosting, CI/CD integration, air-gapped installations, and compliance requirements.

## Table of Contents

- [Use Cases](#use-cases)
- [Private Index Hosting](#private-index-hosting)
- [Custom Signing Keys](#custom-signing-keys)
- [CI/CD Integration](#cicd-integration)
- [Air-Gapped Installations](#air-gapped-installations)
- [Multi-Prefix Environments](#multi-prefix-environments)
- [Audit Logging](#audit-logging)
- [Compliance](#compliance)
- [Performance at Scale](#performance-at-scale)

## Use Cases

### Internal Development Teams

- **Standardized tooling**: Ensure all developers have the same package versions
- **Reproducible builds**: Lock files guarantee consistent environments
- **Fast onboarding**: New developers get productive in minutes

### CI/CD Pipelines

- **Cached dependencies**: Offline mirrors eliminate network latency
- **Deterministic builds**: Pinned versions prevent "works on my machine"
- **Parallel execution**: Multiple jobs share the same package cache

### Regulated Industries

- **Audit trails**: Track every package installation
- **Approved packages**: Curate allowed packages via private index
- **Vulnerability management**: Automated scanning with `brewx audit`

### Air-Gapped Environments

- **Offline operation**: Full functionality without internet access
- **Controlled updates**: Manual mirror synchronization
- **Security compliance**: No external network dependencies

## Private Index Hosting

Host your own brewx index for complete control over available packages.

### Option 1: GitHub (Private Repository)

The simplest approach for teams already using GitHub:

```bash
# Fork the brewx-index repository to your organization
# Make it private
# Update brewx configuration
```

**~/.brewx/config.toml:**
```toml
[index]
base_url = "https://raw.githubusercontent.com/YOUR_ORG/brewx-index/main"
```

For authentication, set a GitHub token:
```bash
export BREWX_GITHUB_TOKEN="ghp_xxxxx"
```

### Option 2: Static File Server

Host the index on any web server (nginx, Apache, S3, etc.):

```bash
# Sync the index locally
cd /path/to/index
./scripts/sync_all.sh

# Structure:
# /path/to/index/
# ├── manifest.json
# ├── index.db.zst
# ├── formulas/
# │   ├── manifest.json
# │   ├── index.db.zst
# │   └── data/
# │       ├── a/
# │       │   └── ack.json.zst
# │       └── ...
# └── casks/
#     └── ...
```

**nginx configuration:**
```nginx
server {
    listen 443 ssl;
    server_name brewx.internal.company.com;

    ssl_certificate /etc/ssl/brewx.crt;
    ssl_certificate_key /etc/ssl/brewx.key;

    root /var/www/brewx-index;

    location / {
        autoindex off;
        add_header Cache-Control "public, max-age=300";
    }

    # Enable gzip for JSON files
    gzip on;
    gzip_types application/json;
}
```

### Option 3: S3 / Cloud Storage

For AWS S3:
```bash
# Sync index to S3
aws s3 sync /path/to/index s3://company-brewx-index/ \
    --content-encoding zstd \
    --cache-control "max-age=300"

# Configure CloudFront for HTTPS
```

**~/.brewx/config.toml:**
```toml
[index]
base_url = "https://brewx.company.cloudfront.net"
```

### Curating Packages

Create a custom index with only approved packages:

```python
#!/usr/bin/env python3
"""Sync only approved formulas to private index."""

import json
from pathlib import Path

APPROVED_FORMULAS = [
    "git", "curl", "jq", "yq", "python@3.11", "node@20",
    "go", "rust", "cmake", "make", "gcc",
    # Add your approved packages
]

def filter_index(source_dir: Path, dest_dir: Path):
    """Copy only approved formulas to destination."""
    for formula in APPROVED_FORMULAS:
        first_char = formula[0]
        src = source_dir / "formulas" / "data" / first_char / f"{formula}.json.zst"
        if src.exists():
            dst = dest_dir / "formulas" / "data" / first_char / f"{formula}.json.zst"
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(src, dst)

    # Rebuild the SQLite index with only approved formulas
    # (Use the sync scripts with a filter)
```

## Custom Signing Keys

Generate and use your own signing keys for complete trust chain control.

### Generate a Keypair

```bash
cd /path/to/brewx-index/scripts

# Install dependencies
uv sync

# Generate new keypair
uv run python sign_index.py generate --output ../keys

# Output:
# Generated keypair:
#   Private key: ../keys/brewx-index.key
#   Public key:  ../keys/brewx-index.pub
#
# Public key (hex): abc123...
```

### Configure brewx to Trust Your Key

**Option 1: Replace the default key** (requires building from source)

Edit `crates/brewx-index/src/signature.rs`:
```rust
pub const DEFAULT_PUBLIC_KEY_HEX: &str = "YOUR_PUBLIC_KEY_HEX";
```

**Option 2: Add as additional trusted key** (runtime configuration)

**~/.brewx/config.toml:**
```toml
[security]
additional_trusted_keys = [
    "YOUR_PUBLIC_KEY_HEX"
]
```

### Sign Your Index

```bash
# Using key file
uv run python sign_index.py sign \
    --key ./keys/brewx-index.key \
    --index-dir /path/to/index

# Using environment variable (for CI)
export BREWX_SIGNING_KEY="private_key_hex"
uv run python sign_index.py sign \
    --key '$BREWX_SIGNING_KEY' \
    --index-dir /path/to/index
```

### Key Management Best Practices

1. **Store private keys securely**: Use HashiCorp Vault, AWS Secrets Manager, or similar
2. **Rotate keys periodically**: Add new keys to `additional_trusted_keys` before rotating
3. **Backup keys**: Losing the private key requires re-signing all indexes
4. **Audit key usage**: Log all signing operations

## CI/CD Integration

### GitHub Actions

```yaml
name: Build with brewx

on: [push, pull_request]

jobs:
  build:
    runs-on: macos-latest  # or ubuntu-latest

    steps:
      - uses: actions/checkout@v4

      - name: Install brewx
        run: |
          curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash
          echo "$HOME/.local/bin" >> $GITHUB_PATH

      - name: Cache brewx packages
        uses: actions/cache@v4
        with:
          path: |
            ~/.brewx/downloads
            ~/.brewx/cache
          key: brewx-${{ runner.os }}-${{ hashFiles('Brewfile.lock') }}
          restore-keys: |
            brewx-${{ runner.os }}-

      - name: Install dependencies
        run: |
          brewx update
          brewx bundle install

      - name: Build
        run: make build
```

### GitLab CI

```yaml
stages:
  - setup
  - build

variables:
  BREWX_CACHE_DIR: ${CI_PROJECT_DIR}/.brewx-cache

install-deps:
  stage: setup
  image: ubuntu:22.04
  cache:
    key: brewx-${CI_COMMIT_REF_SLUG}
    paths:
      - .brewx-cache/
  script:
    - curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash
    - export PATH="$HOME/.local/bin:$PATH"
    - brewx update
    - brewx bundle install
  artifacts:
    paths:
      - /opt/homebrew/Cellar/

build:
  stage: build
  needs: [install-deps]
  script:
    - make build
```

### Jenkins

```groovy
pipeline {
    agent any

    environment {
        BREWX_CACHE = "${WORKSPACE}/.brewx-cache"
    }

    stages {
        stage('Setup') {
            steps {
                sh '''
                    curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash
                    export PATH="$HOME/.local/bin:$PATH"
                    brewx update
                    brewx bundle install
                '''
            }
        }

        stage('Build') {
            steps {
                sh 'make build'
            }
        }
    }

    post {
        always {
            // Cache cleanup
            sh 'brewx cleanup --prune=7'
        }
    }
}
```

### Brewfile for CI

Create a `Brewfile` for reproducible dependencies:

```ruby
# Brewfile
# Build tools
brew "cmake"
brew "make"
brew "ninja"

# Languages
brew "python@3.11"
brew "node@20"
brew "go"
brew "rust"

# Utilities
brew "jq"
brew "yq"
brew "curl"
brew "git"

# Testing
brew "shellcheck"
brew "hadolint"
```

### Lock Files

Generate and commit lock files for reproducibility:

```bash
# Generate lock file from installed packages
brewx lock generate

# Install exact versions from lock file
brewx lock install

# Verify lock file matches installed
brewx lock verify
```

**Brewfile.lock format:**
```json
{
  "generated_at": "2024-11-27T12:00:00Z",
  "brewx_version": "0.1.0",
  "packages": {
    "jq": {
      "version": "1.7.1",
      "bottle_sha256": "abc123...",
      "dependencies": ["oniguruma"]
    }
  }
}
```

## Air-Gapped Installations

For environments without internet access.

### Create an Offline Mirror

```bash
# Create mirror with specific packages
brewx mirror create /path/to/mirror jq curl git python@3.11

# Include all dependencies
brewx mirror create /path/to/mirror --with-deps jq curl

# Mirror everything in a Brewfile
brewx mirror create /path/to/mirror --from-brewfile ./Brewfile

# Include casks
brewx mirror create /path/to/mirror --casks firefox visual-studio-code
```

### Transfer to Air-Gapped System

```bash
# Create archive
tar -czvf brewx-mirror.tar.gz /path/to/mirror

# Transfer via approved method (USB, secure file transfer, etc.)
# On air-gapped system:
tar -xzvf brewx-mirror.tar.gz -C /opt/
```

### Configure for Offline Use

**~/.brewx/config.toml:**
```toml
[index]
# Point to local mirror
base_url = "file:///opt/brewx-mirror"

[security]
# May need to adjust for offline use
allow_unsigned = true  # Or use your own signing key
```

### Serve Mirror on Internal Network

```bash
# Start HTTP server
brewx mirror serve /opt/brewx-mirror --port 8080 --bind 0.0.0.0

# Or use nginx/Apache for production
```

### Update Mirror

```bash
# On internet-connected system
brewx mirror update /path/to/mirror

# Verify integrity
brewx mirror verify /path/to/mirror

# Transfer updated files
rsync -av /path/to/mirror/ user@airgapped:/opt/brewx-mirror/
```

### Mirror Security Options

Mirrors support two security models:

**Option 1: Preserve Upstream Signatures (Recommended)**

The mirror preserves the original official signature:

```bash
# Create mirror - upstream signature is automatically preserved
brewx mirror create /path/to/mirror jq curl git

# The manifest.json will include:
# {
#   "upstream_signature": {
#     "signature": "original_ed25519_sig...",
#     "index_sha256": "original_hash...",
#     "signed_at": 1732723200
#   }
# }
```

Clients verify against the official brewx public key.

**Option 2: Enterprise Re-signing**

Sign the mirror with your own key for full control:

```bash
# Generate enterprise keypair (one-time)
cd /path/to/signing-tools
uv run python sign_index.py generate --output ./keys
# Save keys/brewx-index.key securely (e.g., HashiCorp Vault)
# Distribute keys/brewx-index.pub to clients

# Sign the mirror
uv run python sign_index.py sign \
    --key ./keys/brewx-index.key \
    --index-dir /path/to/mirror

# Configure clients
# ~/.brewx/config.toml
[security]
additional_trusted_keys = ["YOUR_ENTERPRISE_PUBLIC_KEY"]
```

**Comparison:**

| Aspect | Preserve Upstream | Enterprise Re-sign |
|--------|-------------------|-------------------|
| Setup complexity | None | Requires key management |
| Trust chain | Official → Mirror → Client | Enterprise → Client |
| Key rotation | Handled by brewx team | Self-managed |
| Custom packages | Not supported | Fully supported |
| Audit trail | Traceable to official | Internal only |

## Multi-Prefix Environments

Isolate dependencies for different projects or teams.

### Project-Specific Prefixes

```bash
# Create isolated environment for project
brewx prefix create ~/projects/api-service/.brewx

# Install project dependencies
brewx --prefix=~/projects/api-service/.brewx bundle install

# Activate in shell
export PATH="$HOME/projects/api-service/.brewx/bin:$PATH"
export BREWX_PREFIX="$HOME/projects/api-service/.brewx"
```

### Team-Shared Prefixes

```bash
# Create shared prefix
sudo brewx prefix create /opt/team-data-science

# Set permissions
sudo chown -R :data-science /opt/team-data-science
sudo chmod -R g+w /opt/team-data-science

# Team members use
brewx --prefix=/opt/team-data-science install pandas numpy scipy
```

### Container Integration

**Dockerfile:**
```dockerfile
FROM ubuntu:22.04

# Install brewx
RUN curl -fsSL https://raw.githubusercontent.com/neul-labs/brewx/main/install.sh | bash

# Create app-specific prefix
RUN brewx prefix create /app/.brewx

# Install dependencies
COPY Brewfile /app/
RUN brewx --prefix=/app/.brewx bundle install

# Add to PATH
ENV PATH="/app/.brewx/bin:$PATH"
ENV BREWX_PREFIX="/app/.brewx"

COPY . /app
WORKDIR /app
CMD ["./start.sh"]
```

## Audit Logging

Track package operations for compliance and debugging.

### Enable Audit Logging

**~/.brewx/config.toml:**
```toml
[audit]
enabled = true
log_file = "/var/log/brewx/audit.log"
log_format = "json"  # or "text"
include_user = true
include_timestamp = true
```

### Log Format

**JSON format:**
```json
{
  "timestamp": "2024-11-27T12:00:00Z",
  "user": "developer",
  "action": "install",
  "packages": ["jq@1.7.1"],
  "success": true,
  "duration_ms": 1234,
  "source": "bottle"
}
```

### Centralized Logging

Send logs to your SIEM or log aggregator:

```bash
# Syslog
logger -t brewx "$(cat /var/log/brewx/audit.log)"

# Fluent Bit / Fluentd
# Configure to tail /var/log/brewx/audit.log

# Datadog
# Use the Datadog agent with log collection
```

## Compliance

### SOC 2

brewx supports SOC 2 compliance through:

1. **Access controls**: Package installation requires appropriate permissions
2. **Audit trails**: All operations logged with timestamps and users
3. **Change management**: Version control via lock files
4. **Vulnerability management**: Built-in `brewx audit` command

### HIPAA

For HIPAA-regulated environments:

1. **Air-gapped operation**: No external network access required
2. **Encrypted storage**: Compatible with encrypted filesystems
3. **Access logging**: Track who installed what and when

### FedRAMP

Federal deployments can use:

1. **Private hosting**: No dependency on external services
2. **Custom signing**: Own cryptographic keys
3. **FIPS compliance**: Uses standard cryptographic algorithms

## Performance at Scale

### Caching Strategies

**Shared Cache for Build Agents:**
```bash
# NFS mount
mount -t nfs cache-server:/brewx-cache /opt/brewx-cache

# Configure brewx
export BREWX_CACHE_DIR=/opt/brewx-cache
```

**Redis Cache for Metadata:**
```toml
[cache]
backend = "redis"
redis_url = "redis://cache-server:6379/0"
```

### Parallel Operations

```bash
# Increase parallel downloads
brewx config set install.parallel_downloads 8

# Parallel builds (source installation)
brewx install -s --jobs=16 large-package
```

### Monitoring

Export metrics for monitoring:

```bash
# Prometheus metrics endpoint
brewx metrics serve --port 9090

# Metrics available:
# - brewx_packages_installed
# - brewx_cache_hit_ratio
# - brewx_download_duration_seconds
# - brewx_install_duration_seconds
```

## Quick Reference

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `BREWX_PREFIX` | Installation prefix | `/opt/homebrew` |
| `BREWX_CACHE_DIR` | Cache directory | `~/.brewx/cache` |
| `BREWX_CONFIG` | Config file path | `~/.brewx/config.toml` |
| `BREWX_INDEX_URL` | Index URL override | (from config) |
| `BREWX_GITHUB_TOKEN` | GitHub auth token | (none) |
| `BREWX_SIGNING_KEY` | Signing key (CI) | (none) |
| `BREWX_LOG_LEVEL` | Log verbosity | `info` |
| `BREWX_NO_COLOR` | Disable colors | `false` |

### CLI Flags for Enterprise

```bash
# Use specific prefix
brewx --prefix=/custom/path install pkg

# Verbose output for debugging
brewx -v install pkg
brewx -vv install pkg  # More verbose

# Skip signature verification (development only)
brewx update --insecure

# Dry run (show what would happen)
brewx install --dry-run pkg

# Force operations
brewx install --force pkg
```

---

For additional enterprise support, contact the maintainers or open a discussion on GitHub.
