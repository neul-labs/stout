# Security Model

brewx implements a defense-in-depth security model to protect against supply chain attacks, man-in-the-middle attacks, and compromised indexes.

## Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        brewx Security Layers                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Layer 1: Transport Security                                             │
│  ├── HTTPS required (TLS 1.2+)                                          │
│  └── Domain validation (trusted hosts only)                             │
│                                                                          │
│  Layer 2: Index Integrity                                                │
│  ├── Ed25519 cryptographic signatures                                   │
│  ├── Signature freshness validation (max age)                           │
│  └── Pinned public keys in binary                                       │
│                                                                          │
│  Layer 3: Package Integrity                                              │
│  ├── SHA256 checksums for all downloads                                 │
│  └── Checksum verification before installation                          │
│                                                                          │
│  Layer 4: Installation Security                                          │
│  ├── Sandboxed extraction                                               │
│  └── No arbitrary code execution during install                         │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

## Ed25519 Index Signatures

Every brewx index is cryptographically signed using Ed25519 signatures. This ensures:

1. **Authenticity**: The index was created by the official brewx-index maintainers
2. **Integrity**: The index has not been tampered with
3. **Freshness**: The signature is recent (prevents replay attacks)

### How It Works

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  Index DB    │────▶│ SHA256 Hash  │────▶│   Sign w/    │
│  (SQLite)    │     │              │     │ Private Key  │
└──────────────┘     └──────────────┘     └──────┬───────┘
                                                  │
                                                  ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   brewx      │◀────│   Verify     │◀────│  manifest    │
│   client     │     │  Signature   │     │   .json      │
└──────────────┘     └──────────────┘     └──────────────┘
```

1. The index database is hashed (SHA256)
2. The hash + metadata is signed with an Ed25519 private key
3. The signature is stored in `manifest.json`
4. brewx verifies the signature using the embedded public key

### Manifest Format

```json
{
  "version": "2024.11.27.1234",
  "index_version": "2024.11.27.1234",
  "index_sha256": "abc123...",
  "index_size": 3145728,
  "formula_count": 7500,
  "cask_count": 5000,
  "signed_at": 1732723200,
  "signature": "ed25519_signature_hex...",
  "created_at": "2024-11-27T12:00:00Z"
}
```

### Signed Data Format

The signature covers a canonical string:
```
brewx-index:v1:{index_sha256}:{signed_at}:{index_version}:{formula_count}:{cask_count}
```

## Security Configuration

brewx security can be configured in `~/.brewx/config.toml`:

```toml
[security]
# Require valid Ed25519 signatures on index updates
# Default: true (release builds), false (debug builds)
require_signature = true

# Allow unsigned indexes (for development/testing only)
# Default: false (release builds), true (debug builds)
allow_unsigned = false

# Maximum age of signature in seconds before rejecting
# Default: 604800 (7 days)
max_signature_age = 604800

# Additional trusted public keys (for key rotation)
# The default brewx-index key is always trusted
additional_trusted_keys = []
```

### Security Policies

brewx supports three security modes:

| Mode | `require_signature` | `allow_unsigned` | Use Case |
|------|---------------------|------------------|----------|
| **Strict** | `true` | `false` | Production (default in release) |
| **Default** | varies | varies | Based on build type |
| **Permissive** | `false` | `true` | Development only |

## Trusted Public Key

The official brewx-index public key is embedded in the binary:

```
e58d628836f72ecc7f6964ba2b70523d7c1c46512441ef8eccf2fa55ad0258f2
```

This key is used to verify all index updates. The corresponding private key is:
- Stored securely in GitHub Secrets
- Used only by the official CI/CD pipeline
- Never distributed or exposed

## Transport Security

### HTTPS Enforcement

In strict mode, brewx:
- Requires HTTPS for remote index URLs
- Enforces TLS 1.2 or higher
- Validates server certificates
- Allows `file://` URLs for local mirrors

### No Domain Restrictions

brewx intentionally does **not** restrict which domains can host indexes. This is because:

1. **Signature verification is the primary security mechanism** - a valid Ed25519 signature proves the data is authentic regardless of where it's hosted
2. **Enterprises need flexibility** - mirrors can be hosted on internal domains, CDNs, or local file systems
3. **Defense in depth** - HTTPS protects against network-level attacks, but the signature protects against compromised servers

```toml
# All of these are valid with proper signatures:
[index]
base_url = "https://raw.githubusercontent.com/neul-labs/brewx-index/main"  # Official
base_url = "https://brewx-mirror.internal.company.com"                      # Enterprise
base_url = "https://cdn.example.com/brewx"                                  # CDN
base_url = "file:///opt/brewx-mirror"                                       # Local
```

## Package Integrity

### Bottle Verification

All downloaded bottles are verified:

1. **SHA256 Checksum**: Verified against the formula's recorded hash
2. **Size Validation**: Ensures the download is complete
3. **Content Verification**: Validates the archive structure

```rust
// Verification happens before any extraction
if computed_hash != expected_hash {
    return Err(Error::ChecksumMismatch);
}
```

### Cask Verification

macOS applications (casks) are verified using:
- SHA256 checksums (when available)
- File size validation
- Code signature validation (macOS Gatekeeper)

## Vulnerability Scanning

brewx includes built-in vulnerability auditing:

```bash
# Scan all installed packages
brewx audit

# Scan specific packages
brewx audit openssl curl

# Update vulnerability database
brewx audit --update
```

The vulnerability database is sourced from:
- OSV (Open Source Vulnerabilities)
- GitHub Security Advisories
- NVD (National Vulnerability Database)

## Mirror Security

Mirrors allow offline or internal hosting of brewx indexes. The security model extends to mirrors:

### How Mirror Security Works

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Mirror Security Model                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  Option 1: Upstream Signature Preservation                               │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │
│  │   Official   │────▶│    Mirror    │────▶│    brewx     │             │
│  │    Index     │     │   (copies    │     │   (verifies  │             │
│  │  (signed)    │     │  signature)  │     │  signature)  │             │
│  └──────────────┘     └──────────────┘     └──────────────┘             │
│                                                                          │
│  Option 2: Enterprise Re-signing                                         │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐             │
│  │   Official   │────▶│  Enterprise  │────▶│    brewx     │             │
│  │    Index     │     │   Mirror     │     │  (verifies   │             │
│  │              │     │ (re-signed)  │     │ enterprise   │             │
│  │              │     │              │     │    key)      │             │
│  └──────────────┘     └──────────────┘     └──────────────┘             │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Option 1: Preserve Upstream Signatures

When creating a mirror, the upstream signature is preserved:

```bash
# Create mirror - signature is automatically copied
brewx mirror create /path/to/mirror jq curl git

# Mirror manifest includes:
# - upstream_signature.signature (original Ed25519 signature)
# - upstream_signature.index_sha256 (original hash)
# - upstream_signature.signed_at (original timestamp)
```

The brewx client verifies the upstream signature even when fetching from a mirror. This ensures:
- The data originally came from the official index
- No tampering occurred during mirroring
- The signature age is still validated

### Option 2: Enterprise Re-signing

Enterprises can sign mirrors with their own keys:

```bash
# Generate enterprise keypair
cd /path/to/brewx-index/scripts
uv run python sign_index.py generate --output ./enterprise-keys

# Create and sign mirror
brewx mirror create /path/to/mirror jq curl git
uv run python sign_index.py sign \
    --key ./enterprise-keys/brewx-index.key \
    --index-dir /path/to/mirror
```

Configure clients to trust the enterprise key:

```toml
# ~/.brewx/config.toml
[security]
additional_trusted_keys = [
    "YOUR_ENTERPRISE_PUBLIC_KEY_HEX"
]
```

### Security Guarantees

| Scenario | Upstream Sig | Mirror Sig | Security Level |
|----------|--------------|------------|----------------|
| Official mirror | ✓ Preserved | Optional | Full (official trust) |
| Enterprise mirror | ✓ Preserved | ✓ Added | Full (dual trust) |
| Custom curated | ✗ N/A | ✓ Required | Enterprise trust only |
| Development | ✗ N/A | ✗ None | Permissive mode only |

### Air-Gapped Mirror Verification

For air-gapped environments:

1. **Create mirror on connected system**: Signature is preserved
2. **Transfer to air-gapped system**: Via approved media
3. **brewx verifies signature**: Using embedded public key
4. **No network required**: Verification is fully offline

```bash
# On air-gapped system
brewx --prefix=/opt/airgap update  # Uses local mirror
# ✓ Signature verified (signed 2h ago)
```

## Enterprise Security

For enterprise deployments, see [ENTERPRISE.md](ENTERPRISE.md) for:
- Private index hosting
- Custom signing keys
- Air-gapped installations
- Audit logging
- Compliance considerations

## Security Best Practices

### For Users

1. **Keep brewx updated**: Security fixes are released regularly
2. **Run `brewx audit`**: Regularly scan for vulnerable packages
3. **Verify the installer**: Check checksums when downloading brewx
4. **Don't disable security**: Avoid `--insecure` flags in production

### For Enterprises

1. **Host your own index**: Full control over package sources
2. **Use your own signing key**: Independent verification chain
3. **Enable audit logging**: Track all package operations
4. **Regular vulnerability scans**: Automate `brewx audit` in CI/CD

## Reporting Security Issues

If you discover a security vulnerability in brewx:

1. **Do not** open a public GitHub issue
2. Email security concerns to the maintainers privately
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

We aim to respond within 48 hours and will coordinate disclosure.

## Cryptographic Details

### Ed25519 Parameters

- **Algorithm**: Ed25519 (RFC 8032)
- **Key size**: 256-bit (32 bytes)
- **Signature size**: 512-bit (64 bytes)
- **Hash function**: SHA-512 (internal to Ed25519)

### Why Ed25519?

- **Fast**: Verification is extremely fast
- **Small**: Compact keys and signatures
- **Secure**: No known practical attacks
- **Deterministic**: Same input always produces same signature
- **Widely supported**: Available in all major crypto libraries

## Comparison with Homebrew

| Feature | brewx | Homebrew |
|---------|-------|----------|
| Index signatures | Ed25519 | None (git commit hashes) |
| Transport security | HTTPS required | HTTPS (via git) |
| Bottle checksums | SHA256 | SHA256 |
| Code signing (macOS) | Gatekeeper | Gatekeeper |
| Vulnerability scanning | Built-in | External tools |
| Offline verification | Yes | No (requires git) |

## Version History

| Version | Changes |
|---------|---------|
| 0.1.0 | Initial security model with Ed25519 signatures |

---

For questions about brewx security, see the [FAQ](USAGE.md#security-faq) or open a discussion on GitHub.
