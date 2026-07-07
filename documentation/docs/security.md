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

- Email: security@neullabs.com
- GitHub: Private security advisory

Please do not open public issues for security vulnerabilities.

---

## Where Security Lives in the Codebase

The security model is split across a handful of crates rather than living in
one monolithic module. When auditing the implementation, these are the
relevant entry points:

| Concern | Crate / module |
|---------|----------------|
| Ed25519 signature verification | `crates/stout-index/` |
| SHA256 bottle / cask verification | `crates/stout-fetch/` |
| Receipt writing and prefix relocation | `crates/stout-install/` |
| CVE database sync and matching | `crates/stout-audit/` |
| Mirror integrity checks | `crates/stout-mirror/` |
| `doctor` health checks (signature scan included) | `src/cli/doctor.rs` |

The CVE database itself is regenerated by `scripts/sync_vulns.py` and
published alongside the formula index.

---

## What `stout doctor` Checks

The doctor command performs more than a connectivity ping. Each run walks
through:

- Presence and permissions of `~/.stout/` and its sub-directories
- Validity of `config.toml` (TOML parse, known keys, sane values)
- Signature freshness of the cached SQLite index
- Drift between `state.db` and the on-disk Cellar / Caskroom
- Unrelocated Homebrew placeholders inside installed bottles
- Mach-O code signatures on every binary in the prefix (macOS only)
- Reachability and TLS posture of the configured `index.base_url`

`stout doctor --fix` is non-interactive: it re-syncs state, re-relocates
placeholders, re-signs binaries where it has the metadata to do so, and
queues reinstalls for anything it cannot repair in place.

---

## Threat Model Summary

Stout assumes the adversary controls the network and may compromise the
hosting layer of the package index. It does **not** assume a compromised
local user account — anyone with write access to `~/.stout/state.db` or the
Cellar can already do anything stout could.

| Threat | Mitigation |
|--------|------------|
| Tampered index served over the network | Ed25519 signature with embedded public key |
| Replay of an older signed index | `max_signature_age` rejection (default 7 days) |
| Tampered bottle artifact | SHA256 checksum bound into the signed index |
| Malicious post-install script | No script execution; extraction is data-only |
| Compromised dependency (CVE) | `stout audit` against synced CVE database |
| Rogue formula in a tap | Taps require explicit opt-in; signatures still enforced |

The threat model is documented in greater depth in
[`docs/SECURITY.md`](https://github.com/neul-labs/stout/blob/main/docs/SECURITY.md)
inside the repository.
