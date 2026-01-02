# FAQ

Frequently asked questions about Stout.

---

## General

### What is Stout?

Stout is a fast, Rust-based package manager that's compatible with Homebrew. It provides the same commands and uses the same package ecosystem, but runs 10-100x faster.

### Why is Stout faster than Homebrew?

Several design choices make Stout faster:

1. **No Ruby** - Native Rust binary starts in 5ms vs 500ms
2. **No Git** - Downloads 2MB index vs 700MB+ Git operations
3. **Pre-computed metadata** - SQLite with FTS5, not runtime Ruby evaluation
4. **Parallel downloads** - Async Tokio runtime for concurrent operations
5. **Smart caching** - Aggressive caching with hash-based invalidation

### Can I use Stout alongside Homebrew?

Yes. Stout uses the same Cellar structure and creates compatible receipt files. Packages installed by either tool are visible to both.

### Does Stout support all Homebrew packages?

Stout supports packages that provide pre-built bottles. Building from source is supported but less tested than Homebrew's build system.

---

## Installation

### How do I install Stout?

The quickest method:

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

See [Installation](getting-started.md) for more options.

### What platforms are supported?

- macOS 12+ (Intel and Apple Silicon)
- Linux (glibc 2.17+, x86_64 and arm64)

### How do I update Stout?

```bash
# If installed via the install script
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash

# If installed via package manager
npm update -g stout-pkg  # or pip, gem, etc.
```

### How do I uninstall Stout?

```bash
rm ~/.local/bin/stout
rm -rf ~/.stout
```

---

## Usage

### How do I search for packages?

```bash
stout search <query>
```

Stout uses full-text search, so partial matches work well.

### How do I install a specific version?

```bash
stout install python@3.11
```

### How do I see what's installed?

```bash
stout list
```

### How do I update all packages?

```bash
stout upgrade
```

### How do I prevent a package from updating?

```bash
stout pin <package>
```

### Where are packages installed?

Same location as Homebrew:

- **macOS:** `/opt/homebrew/Cellar`
- **Linux:** `/home/linuxbrew/.linuxbrew/Cellar`

---

## Compatibility

### Can I use my existing Brewfile?

Yes:

```bash
stout bundle install
```

### Are Homebrew taps supported?

Tap support is available:

```bash
stout tap user/repo
```

### Do post-install scripts run?

No. Stout doesn't execute Ruby scripts. If a package requires post-install setup, you'll need to run it manually. This is a security feature.

### What about caveats?

Caveats (post-install instructions) are displayed after installation, same as Homebrew.

---

## Casks

### What are casks?

Casks are macOS applications (like Firefox, VS Code) and Linux desktop apps.

### How do I install a cask?

```bash
stout cask install firefox
```

### Where are applications installed?

- **macOS:** `/Applications` or `~/Applications`
- **Linux:** `~/.local/share/appimages`

---

## Security

### How does Stout verify packages?

Two layers of verification:

1. **Index:** Ed25519 cryptographic signatures
2. **Packages:** SHA256 checksums

### Is the index signed?

Yes. The package index is signed with Ed25519. Stout refuses to use an index with an invalid signature.

### Can I use a private index?

Yes. See [Enterprise](enterprise.md) for details on hosting your own package index.

### How do I check for vulnerabilities?

```bash
stout audit
```

---

## Troubleshooting

### "Command not found: stout"

Add the installation directory to your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add this to your shell's rc file (`~/.bashrc`, `~/.zshrc`, etc.)

### "Signature verification failed"

The package index signature is invalid. This could mean:

- Network corruption
- Index has been tampered with
- Stale cache

Try:

```bash
rm -rf ~/.stout/cache
stout update --force
```

### "Package not found"

Update your package index:

```bash
stout update
```

If still not found, the package may not exist or may be in a tap:

```bash
stout search <partial-name>
```

### Installation fails with permission error

Check that you own the Cellar directory:

```bash
sudo chown -R $(whoami) /opt/homebrew
```

Or use a custom prefix:

```bash
stout prefix create ~/.local/stout
stout --prefix=~/.local/stout install <package>
```

### "Too many open files"

Increase your ulimit:

```bash
ulimit -n 10240
```

Make permanent by adding to shell rc file.

---

## Configuration

### Where is the config file?

`~/.stout/config.toml`

### How do I change the number of parallel downloads?

```toml
[install]
parallel_downloads = 8
```

### How do I use a mirror?

```bash
stout --mirror=http://mirror.example.com install jq
```

Or in config:

```toml
[mirror]
url = "http://mirror.example.com"
```

---

## Development

### How do I build Stout from source?

```bash
git clone https://github.com/neul-labs/stout.git
cd stout
cargo build --release
```

### How do I contribute?

See the [Contributing Guide](https://github.com/neul-labs/stout/blob/main/CONTRIBUTING.md).

### Where do I report bugs?

Open an issue at [github.com/neul-labs/stout/issues](https://github.com/neul-labs/stout/issues).
