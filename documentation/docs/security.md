# Security

Stout implements defense-in-depth security to protect your system.

---

## Security Model Overview

Stout's security is built on multiple layers:

1. **Transport Security** - HTTPS with TLS 1.2+
2. **Index Integrity** - Ed25519 cryptographic signatures
3. **Package Integrity** - SHA256 checksums
4. **Installation Security** - Sandboxed extraction

---

## Transport Security

### HTTPS Required

All network operations use HTTPS:

- Package index downloads
- Bottle (binary package) downloads
- Cask downloads

HTTP URLs are automatically upgraded to HTTPS.

### TLS Requirements

- Minimum TLS 1.2
- Modern cipher suites only
- Certificate validation enforced

---

## Index Integrity

### Ed25519 Signatures

The package index is cryptographically signed using Ed25519:

- Every index update includes a signature
- Signatures are verified before the index is used
- The public key is embedded in the Stout binary

### Signature Verification

```
Index File → SHA256 Hash → Ed25519 Verify → Public Key
```

If verification fails, Stout refuses to use the index.

### Signature Freshness

Signatures include a timestamp. Stout rejects signatures older than the configured maximum age (default: 7 days):

```toml
[security]
max_signature_age = 604800  # 7 days in seconds
```

This prevents replay attacks with old, potentially compromised indexes.

---

## Package Integrity

### SHA256 Checksums

Every package download is verified:

1. Index contains expected SHA256 hash
2. Package is downloaded
3. SHA256 hash is computed
4. Hashes are compared

If hashes don't match, installation is aborted.

### Verification Flow

```
Download → Compute SHA256 → Compare with Index → Install or Abort
```

---

## Installation Security

### Sandboxed Extraction

Package extraction is sandboxed:

- No arbitrary code execution during install
- Files extracted only to designated directories
- Symlinks validated before creation

### No Post-Install Scripts

Unlike Homebrew, Stout doesn't execute Ruby scripts during installation. Package installation is purely file extraction and symlinking.

---

## Configuration

### Security Settings

```toml
[security]
# Require signatures (recommended: true)
require_signature = true

# Allow unsigned packages (recommended: false)
allow_unsigned = false

# Maximum signature age in seconds
max_signature_age = 604800

# Additional trusted public keys
trusted_keys = []
```

### Recommended Settings

For production environments:

```toml
[security]
require_signature = true
allow_unsigned = false
max_signature_age = 604800
```

!!! danger "Never Disable Signature Verification"
    Setting `require_signature = false` or `allow_unsigned = true` removes critical security protections. Only do this in isolated development environments.

---

## Trust Model

### Default Trust

Stout trusts:

1. The embedded Neul Labs public key
2. Any additional keys in `trusted_keys` configuration
3. HTTPS certificates from standard CA roots

### Custom Trust (Enterprise)

For private indexes, add your organization's public key:

```toml
[security]
trusted_keys = [
    "your-base64-encoded-ed25519-public-key"
]
```

Generate a key pair:

```bash
# Generate private key
openssl genpkey -algorithm ED25519 -out private.pem

# Extract public key
openssl pkey -in private.pem -pubout -out public.pem

# Convert to base64 for config
base64 -w0 public.pem
```

---

## Vulnerability Scanning

### Audit Command

Scan installed packages for known vulnerabilities:

```bash
stout audit
```

### Filter by Severity

```bash
stout audit --severity=high
stout audit --severity=critical
```

### Severity Levels

| Level | Description |
|-------|-------------|
| `low` | Minor issues, low impact |
| `medium` | Moderate issues |
| `high` | Serious vulnerabilities |
| `critical` | Severe, actively exploited |

### CVE Database

Stout maintains a vulnerability database synced from public sources. Update it with:

```bash
stout update
```

---

## Comparison with Homebrew

| Feature | Homebrew | Stout |
|---------|----------|-------|
| Transport | HTTPS | HTTPS |
| Index integrity | Git commit hashes | Ed25519 signatures |
| Package integrity | SHA256 | SHA256 |
| Post-install scripts | Ruby execution | None |
| Signature verification | No | Yes |
| CVE scanning | No | Built-in |

---

## Security Best Practices

### Keep Stout Updated

```bash
# Update stout itself
stout upgrade stout
```

### Regular Audits

```bash
# Weekly vulnerability scan
stout audit
```

### Review Before Install

Check package details before installing:

```bash
stout info suspicious-package
```

### Use Pinning for Stability

Pin critical packages to prevent unexpected updates:

```bash
stout pin openssl
stout pin python@3.11
```

### Monitor Outdated Packages

Old packages may have unpatched vulnerabilities:

```bash
stout outdated
```

---

## Reporting Security Issues

Report security vulnerabilities to:

- Email: security@neul-labs.com
- GitHub: Private security advisory

Please do not open public issues for security vulnerabilities.
