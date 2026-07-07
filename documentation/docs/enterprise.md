# Enterprise Features

Stout includes features designed for enterprise environments.

---

## Private Index Hosting

Host your own package index for internal packages or to control available software.

### Setup

1. **Fork the stout-index repository**

   ```bash
   git clone https://github.com/neul-labs/stout-index.git
   cd stout-index
   ```

2. **Add your formulas**

   Add formula JSON files to the index.

3. **Sign the index**

   Generate a signing key and sign the index:

   ```bash
   # Generate key pair
   openssl genpkey -algorithm ED25519 -out signing-key.pem

   # Sign the index
   ./scripts/sign-index.py --key signing-key.pem
   ```

4. **Host the index**

   Options:
   - GitHub (public or private repository)
   - S3 bucket
   - Any static file server (nginx, Apache)
   - Internal package registry

5. **Configure clients**

   ```toml
   [index]
   base_url = "https://packages.internal.company.com/stout-index"

   [security]
   trusted_keys = ["your-public-key-base64"]
   ```

### Self-Signed Index

For internal use, you can generate and trust your own signing keys:

```bash
# Generate key pair
openssl genpkey -algorithm ED25519 -out private.pem
openssl pkey -in private.pem -pubout -out public.pem

# Get base64 public key for config
cat public.pem | base64 -w0
```

Add to client configuration:

```toml
[security]
trusted_keys = ["base64-encoded-public-key-here"]
```

---

## Multi-Prefix Environments

Isolate package installations per project or team.

### Create a Prefix

```bash
stout prefix create ~/projects/webapp/.stout
```

### Install to Prefix

```bash
stout --prefix=~/projects/webapp/.stout install node@20 python@3.11
```

### Set Default Prefix

```bash
stout prefix default ~/projects/webapp/.stout
```

### List Prefixes

```bash
stout prefix list
```

### Environment Variable

You can also set the default prefix via environment variable:

```bash
export STOUT_PREFIX=~/projects/myapp/.stout
stout install node@20  # Installs to custom prefix
```

### Use Cases

- **Project isolation:** Each project has its own dependencies
- **Team environments:** Shared dependencies for a team
- **Version testing:** Test different versions side-by-side
- **Reproducible builds:** Lock down exact package versions

---

## Air-Gapped Deployments

Run Stout in environments without internet access.

### Create a Mirror

On a connected machine:

```bash
stout mirror create ./mirror jq curl wget git python@3.11 node@20
```

This downloads:
- Package index
- All specified packages and their dependencies
- Checksums and signatures

### Transfer to Air-Gapped Environment

Copy the mirror directory to the isolated environment:

```bash
# USB drive, sneakernet, etc.
cp -r ./mirror /mnt/usb/stout-mirror
```

### Serve Locally

On the air-gapped machine:

```bash
stout mirror serve /mnt/usb/stout-mirror --port 9000
```

### Configure Clients

```toml
[index]
base_url = "http://localhost:9000"
auto_update = false
```

Or use file:// URLs:

```bash
stout --mirror=file:///mnt/usb/stout-mirror install jq
```

### Verify Mirror Integrity

```bash
stout mirror verify /mnt/usb/stout-mirror
```

---

## CI/CD Integration

### Lock Files

Create reproducible builds with lock files:

```bash
# Generate lock file
stout lock generate

# Install from lock file
stout lock install
```

Lock files include:
- Exact package versions
- SHA256 checksums
- Dependency tree

### Brewfile Support

Use existing Brewfiles:

```bash
stout bundle install
stout bundle check
```

### CI Configuration Examples

**GitHub Actions:**

```yaml
- name: Install dependencies
  run: |
    curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
    stout bundle install
```

**GitLab CI:**

```yaml
before_script:
  - curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
  - stout bundle install
```

### Caching

Cache the stout directory for faster CI:

```yaml
# GitHub Actions
- uses: actions/cache@v3
  with:
    path: ~/.stout/cache
    key: stout-${{ hashFiles('Brewfile.lock') }}
```

### Quiet Mode for CI

```bash
stout --quiet bundle install
```

Or in configuration:

```toml
[output]
color = false
progress = false
verbosity = "quiet"
```

---

## Audit Logging

Track package operations for compliance.

### Enable Audit Logging

```toml
[audit]
enabled = true
log_path = "/var/log/stout/audit.log"
```

### Log Format

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "action": "install",
  "package": "jq",
  "version": "1.7.1",
  "user": "developer",
  "success": true
}
```

### Integration with SIEM

Forward logs to your security information and event management system:

```bash
# Example: Forward to syslog
tail -f /var/log/stout/audit.log | logger -t stout
```

---

## Approved Package Lists

Restrict which packages can be installed.

### Configuration

```toml
[policy]
# Only allow these packages
allowed_packages = [
    "git",
    "node@20",
    "python@3.11",
    "jq",
]

# Or block specific packages
blocked_packages = [
    "dangerous-package",
]
```

### Enforcement

Attempts to install non-approved packages fail:

```bash
$ stout install unapproved-pkg
Error: Package 'unapproved-pkg' is not in the approved list
```

---

## Deployment Strategies

### Centralized Management

1. Host private index
2. Distribute configuration via configuration management (Ansible, Puppet, Chef)
3. Use approved package lists
4. Enable audit logging

### Decentralized with Standards

1. Provide base configuration template
2. Allow project-specific prefixes
3. Require lock files for production
4. Audit via CI/CD pipelines

---

## Support

For enterprise support inquiries:

- Email: enterprise@neullabs.com
- Documentation: <https://docs.neullabs.com/stout/enterprise/>

---

## Lockfiles and Reproducible Rollouts

`stout lock` produces a `stout.lock` file that pins every package — including
transitive dependencies — to an exact version and SHA256 checksum. The
`stout-bundle` crate parses this file and `stout lock install` replays it
deterministically.

A typical CI shape for fleet-wide rollouts:

```bash
# Once, on a maintainer's machine — commit Brewfile + stout.lock together
stout bundle dump > Brewfile
stout lock generate > stout.lock
git add Brewfile stout.lock && git commit -m "Pin dev environment"
```

```bash
# In CI / on every workstation
stout update                  # refresh the index (no version drift; lock pins versions)
stout lock install            # install the exact pinned set
stout doctor                  # validate the install before declaring success
```

Lock files are forward-compatible: an old `stout.lock` will still install on
a newer stout binary as long as the pinned bottles remain available in the
index (or a mirror). For long-lived rollouts, mirror the bottles alongside
the lockfile so availability never depends on upstream Homebrew.

---

## Policy Enforcement Surface

The `[policy]` table is enforced inside `stout-resolve` before the resolver
even contacts the network, so blocked packages fail fast and never appear in
caches or mirrors. Enforcement covers:

- Explicit installs (`stout install`)
- Transitive dependencies pulled in by an allowed package
- `stout bundle install` and `stout lock install`
- Mirror creation (`stout mirror create` refuses to bundle blocked packages)

Combine `allowed_packages` (whitelist) and `blocked_packages` (blacklist)
as needed; the whitelist takes precedence when both are present.

---

## Centralised Audit Logging

When `[audit] enabled = true` is set, every state-changing operation
appends a JSON line to the configured log path. The schema is stable:

```json
{
  "timestamp": "2026-06-05T12:00:00Z",
  "action": "install",
  "package": "jq",
  "version": "1.7.1",
  "user": "developer",
  "prefix": "/opt/homebrew",
  "success": true,
  "duration_ms": 412
}
```

Pipe it into journald, syslog, Splunk, or any line-oriented SIEM:

```bash
# Forward to journald
tail -F /var/log/stout/audit.log | systemd-cat -t stout

# Forward to a syslog collector
tail -F /var/log/stout/audit.log | logger -t stout -n collector.internal -P 514
```

---

## Rolling Out via Configuration Management

For Ansible-style fleets, ship the binary and a templated `config.toml`:

```yaml
- name: Install stout
  ansible.builtin.shell: |
    curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
  environment:
    STOUT_VERSION: "v0.2.2"
    STOUT_INSTALL_DIR: "/usr/local/bin"
    STOUT_NO_MODIFY_PATH: "1"

- name: Drop org config
  ansible.builtin.template:
    src: stout-config.toml.j2
    dest: "/etc/stout/config.toml"
    mode: "0644"

- name: Point users at it
  ansible.builtin.lineinfile:
    path: /etc/environment
    line: 'STOUT_CONFIG=/etc/stout/config.toml'
```

`STOUT_CONFIG` overrides the per-user config path, which makes it easy to
ship one organisation-wide policy file without touching every home directory.
