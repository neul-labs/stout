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

---

## How the Mirror is Laid Out

The `stout-mirror` crate produces a self-describing directory. Every
`stout mirror create` writes the same shape regardless of size:

```
my-mirror/
├── manifest.json          # mirror metadata (created_at, generator version)
├── index/
│   ├── formulas.db        # SQLite formula index (FTS5 inside)
│   ├── formulas.db.sig    # Ed25519 signature over formulas.db
│   ├── casks.db           # cask index
│   └── casks.db.sig
├── bottles/
│   └── <name>-<ver>.<platform>.bottle.tar.gz
├── casks/
│   └── <name>-<ver>.<artifact>
└── vulns/
    └── cve.db             # optional: included when audit data is mirrored
```

`stout mirror serve` simply exposes this tree over HTTP without any
rewriting, which is why nginx, S3 website hosting, or any plain static
file server works as a drop-in replacement once the directory is staged.

---

## Refreshing Without Re-Downloading Everything

The `--from` flag re-uses an existing mirror as a cache when constructing a
new one. Bottles whose SHA256 already matches the index are linked rather
than re-fetched:

```bash
stout mirror create ./mirror-2026-06 \
    --from ./mirror-2026-05 \
    $(cat /etc/stout/approved-packages.txt)
```

Pair this with a cron job (or scheduled GitHub Action) to keep an offline
mirror current with bounded bandwidth.

---

## Verification Walkthrough

`stout mirror verify` is exhaustive and is the single command you want in a
post-deploy validation step:

1. Parse `manifest.json` and confirm the generator version matches.
2. Verify the Ed25519 signature on each index file against the trusted keys.
3. Walk every bottle / cask listed in the index and confirm:
    - the file is present at the expected path,
    - its SHA256 matches the value baked into the signed index,
    - the file size matches the declared size.
4. (Optional) verify CVE database integrity when `vulns/` is present.

Any failure aborts with a non-zero exit code listing every affected file.
This is the right place to gate a deployment: if `stout mirror verify`
fails, do not flip the load balancer.

---

## Mirroring via Static Hosting (S3, GCS, B2)

Because the mirror directory is purely static, you can publish it to object
storage and front it with a CDN:

```bash
# Build locally
stout mirror create ./mirror $(cat approved-packages.txt)
stout mirror verify ./mirror

# Sync to S3 with content-type hints
aws s3 sync ./mirror s3://my-stout-mirror/ \
    --delete \
    --content-type-by-extension \
    --metadata-directive REPLACE
```

Then point clients at the CDN edge:

```toml
[index]
base_url = "https://stout-mirror.cdn.example.com/index"

[mirror]
url = "https://stout-mirror.cdn.example.com"
```

Because every file in the mirror is signed or checksummed, hosting it on
untrusted infrastructure is safe — tampering will surface as a verification
failure on the client.
