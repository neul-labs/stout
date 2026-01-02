# Offline & Mirroring

Deploy Stout in air-gapped and offline environments.

---

## Overview

Stout's mirroring feature allows you to:

- Create offline copies of packages
- Serve packages on local networks
- Deploy in air-gapped environments
- Reduce external bandwidth usage

---

## Creating a Mirror

### Basic Mirror

Create a mirror with specific packages:

```bash
stout mirror create ./my-mirror jq curl wget
```

This downloads:
- The package index
- Specified packages
- All dependencies
- Checksums and signatures

### Include Dependencies

Dependencies are automatically included:

```bash
stout mirror create ./mirror python@3.11
# Includes: python@3.11, openssl, readline, sqlite, xz, etc.
```

### Large Mirror

Create a comprehensive development mirror:

```bash
stout mirror create ./dev-mirror \
    git \
    python@3.11 \
    node@20 \
    rust \
    go \
    jq \
    curl \
    wget \
    vim \
    neovim
```

---

## Mirror Contents

A mirror contains:

```
my-mirror/
├── index/
│   ├── formulas.db          # SQLite index
│   ├── formulas.db.sig      # Index signature
│   └── casks.db              # Cask index
├── bottles/
│   ├── jq-1.7.1.arm64_monterey.bottle.tar.gz
│   ├── curl-8.4.0.arm64_monterey.bottle.tar.gz
│   └── ...
├── casks/
│   └── ...
└── manifest.json             # Mirror manifest
```

---

## Serving a Mirror

### Built-in HTTP Server

Serve the mirror over HTTP:

```bash
stout mirror serve ./my-mirror --port 9000
```

Access at `http://localhost:9000`

### Custom Host Binding

```bash
stout mirror serve ./my-mirror --host 0.0.0.0 --port 9000
```

### Behind a Reverse Proxy

For production, use nginx or similar:

```nginx
server {
    listen 443 ssl;
    server_name packages.internal.company.com;

    location / {
        root /var/www/stout-mirror;
        autoindex on;
    }
}
```

---

## Using a Mirror

### Command Line

```bash
stout --mirror=http://localhost:9000 install jq
```

### Configuration

```toml
[index]
base_url = "http://localhost:9000/index"

[mirror]
url = "http://localhost:9000"
```

### File-based Mirror

For truly offline environments:

```bash
stout --mirror=file:///mnt/usb/stout-mirror install jq
```

---

## Mirror Management

### View Mirror Info

```bash
stout mirror info ./my-mirror
```

Output:
```
Mirror: ./my-mirror
Created: 2024-01-15T10:30:00Z
Packages: 45
Size: 1.2 GB

Packages:
  jq 1.7.1
  curl 8.4.0
  wget 1.21.4
  ...
```

### Verify Mirror Integrity

```bash
stout mirror verify ./my-mirror
```

Checks:
- All files present
- SHA256 checksums match
- Signatures valid

### Update Mirror

Add new packages to an existing mirror:

```bash
stout mirror create ./my-mirror --update new-package
```

---

## Air-Gapped Workflow

### 1. Create Mirror (Connected Machine)

```bash
stout mirror create ./transfer-mirror python@3.11 node@20
```

### 2. Transfer to Air-Gapped Environment

```bash
# Copy to USB drive
cp -r ./transfer-mirror /mnt/usb/

# Or create archive
tar -czf stout-mirror.tar.gz ./transfer-mirror
```

### 3. Deploy on Air-Gapped Machine

```bash
# Extract if archived
tar -xzf stout-mirror.tar.gz

# Serve locally
stout mirror serve ./transfer-mirror --port 9000 &

# Configure stout
cat > ~/.stout/config.toml << EOF
[index]
base_url = "http://localhost:9000/index"
auto_update = false

[mirror]
url = "http://localhost:9000"
EOF
```

### 4. Install Packages

```bash
stout install python@3.11 node@20
```

---

## CI/CD with Mirrors

### Self-Hosted Mirror for Build Servers

1. **Create mirror on a scheduled job:**

   ```bash
   # Weekly job to update mirror
   stout mirror create /var/www/stout-mirror \
       $(cat /etc/stout/approved-packages.txt)
   ```

2. **Configure build agents:**

   ```toml
   [index]
   base_url = "http://build-mirror.internal:9000/index"

   [mirror]
   url = "http://build-mirror.internal:9000"
   ```

3. **Faster, reproducible builds:**

   - No external network calls
   - Consistent package versions
   - Reduced bandwidth

---

## Bandwidth Optimization

### Mirror Specific Architectures

Only mirror what you need:

```bash
stout mirror create ./mirror --arch arm64 --os monterey python@3.11
```

### Exclude Development Dependencies

```bash
stout mirror create ./mirror --no-dev-deps production-package
```

---

## Troubleshooting

### Mirror Verification Failed

```bash
$ stout mirror verify ./my-mirror
Error: Checksum mismatch for jq-1.7.1.bottle.tar.gz
```

Re-download the affected package:

```bash
stout mirror create ./my-mirror --force jq
```

### Package Not Found in Mirror

```bash
$ stout install missing-package
Error: Package 'missing-package' not found in mirror
```

Add the package to the mirror on a connected machine and re-transfer.

### Signature Expired

Signatures have a maximum age. Re-create the mirror to get fresh signatures:

```bash
stout mirror create ./new-mirror --from ./old-mirror
```
