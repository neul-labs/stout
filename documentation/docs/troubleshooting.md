# Troubleshooting

Solutions to common problems with Stout.

---

## Installation Issues

### "Command not found: stout"

Stout isn't in your PATH.

**Solution:**

```bash
# Add to PATH
export PATH="$HOME/.local/bin:$PATH"

# Add to your shell config file
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc
```

### "Permission denied" during installation

The installation directory isn't writable.

**Solution:**

```bash
# Option 1: Fix permissions on Homebrew directory
sudo chown -R $(whoami) /opt/homebrew

# Option 2: Use a local prefix
stout prefix create ~/.stout
stout --prefix=~/.stout install <package>
```

### "Binary not found for your platform"

No pre-built binary for your system.

**Solution:**

Build from source:

```bash
cargo install stout
```

---

## Update Issues

### "Signature verification failed"

The index signature is invalid or expired.

**Solution:**

```bash
# Clear cache and force update
rm -rf ~/.stout/cache
stout update --force
```

If problem persists, check your system clock is correct.

### "Failed to download index"

Network connectivity issue.

**Solution:**

```bash
# Check connectivity
curl -I https://raw.githubusercontent.com/neul-labs/stout-index/main/index.json

# Use a mirror if available
stout --mirror=http://internal-mirror.company.com update
```

### "Index too old"

Your local index hasn't been updated recently.

**Solution:**

```bash
stout update
```

---

## Package Installation Issues

### "Package not found"

The package doesn't exist or isn't indexed.

**Solution:**

```bash
# Update index
stout update

# Search for the package
stout search <partial-name>

# Check if it's in a tap
stout tap neul-labs/extra
stout search <package>
```

### "Checksum mismatch"

Downloaded file doesn't match expected hash.

**Solution:**

```bash
# Clear download cache
stout cleanup -s

# Retry installation
stout install <package>
```

If problem persists, the package may have been updated. Run `stout update`.

### "Bottle not available"

No pre-built binary for your platform/version.

**Solution:**

```bash
# Build from source
stout install --build-from-source <package>
```

### "Dependency conflict"

Two packages require incompatible dependency versions.

**Solution:**

```bash
# Check dependencies
stout deps --tree <package>

# Try installing without conflicting package
stout uninstall <conflicting-package>
stout install <package>
```

### "Disk space full"

Not enough space for installation.

**Solution:**

```bash
# Clean old downloads and versions
stout cleanup

# Check what's using space
stout list --versions

# Remove unused dependencies
stout autoremove
```

---

## Runtime Issues

### "Too many open files"

Hit system file descriptor limit.

**Solution:**

```bash
# Increase limit for current session
ulimit -n 10240

# Make permanent (add to shell rc file)
echo 'ulimit -n 10240' >> ~/.bashrc
```

### "Library not found"

Dynamically linked library is missing.

**Solution:**

```bash
# Reinstall the package
stout reinstall <package>

# Or reinstall dependencies
stout reinstall $(stout deps <package>)
```

### "Command not working after install"

Symlinks may not have been created.

**Solution:**

```bash
# Relink the package
stout unlink <package>
stout link <package>
```

---

## Cask Issues

### "Application can't be opened (macOS)"

macOS Gatekeeper is blocking the app.

**Solution:**

1. Open System Preferences > Security & Privacy
2. Click "Open Anyway" for the blocked app

Or remove quarantine:

```bash
xattr -d com.apple.quarantine /Applications/<app>.app
```

### "Cask install fails with existing app"

Application already installed outside of Stout.

**Solution:**

```bash
# Remove existing app first
rm -rf /Applications/<app>.app

# Or force install
stout cask install --force <cask>
```

### "Zap leaves files behind"

Some files weren't identified for removal.

**Solution:**

Check common locations manually:

```bash
# Application Support
ls ~/Library/Application\ Support/<app>

# Preferences
ls ~/Library/Preferences/com.<app>.*

# Caches
ls ~/Library/Caches/<app>
```

---

## Performance Issues

### "Stout is slow"

First run is slower while building cache.

**Solution:**

```bash
# Update index and warm cache
stout update
stout list > /dev/null
```

### "Downloads are slow"

Network or server issues.

**Solution:**

```bash
# Check parallel download setting
stout config

# Increase parallel downloads
cat >> ~/.stout/config.toml << EOF
[install]
parallel_downloads = 8
EOF
```

---

## Configuration Issues

### "Config file not loading"

Syntax error in TOML file.

**Solution:**

```bash
# Validate TOML syntax
cat ~/.stout/config.toml

# Check for common issues:
# - Missing quotes around strings
# - Wrong indentation
# - Invalid values

# Reset to defaults
rm ~/.stout/config.toml
```

### "Environment variable not working"

Variable may not be exported.

**Solution:**

```bash
# Make sure to export
export STOUT_PREFIX=/custom/path

# Not just
STOUT_PREFIX=/custom/path  # This doesn't work
```

---

## Multi-Prefix Issues

### "Wrong prefix being used"

Default prefix may be set incorrectly.

**Solution:**

```bash
# Check current default
stout prefix list

# Set correct default
stout prefix default /path/to/prefix

# Or specify explicitly
stout --prefix=/path/to/prefix install <package>
```

---

## Diagnostic Commands

Use these commands to gather information for troubleshooting:

```bash
# System health check
stout doctor

# Show configuration
stout config

# Show version
stout --version

# Verbose output
stout --verbose install <package>

# Check what's installed
stout list --versions
```

---

## Getting Help

If these solutions don't resolve your issue:

1. **Search existing issues:** [github.com/neul-labs/stout/issues](https://github.com/neul-labs/stout/issues)

2. **Open a new issue** with:
   - Stout version (`stout --version`)
   - Operating system and version
   - Complete error message
   - Steps to reproduce
   - Output of `stout doctor`

3. **Community help:** Discussions on GitHub
