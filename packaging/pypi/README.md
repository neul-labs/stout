# stout

A fast, Rust-based Homebrew-compatible package manager - **10-100x faster** than brew.

## Installation

```bash
pip install stout-pkg
```

Or with pipx for isolated installation:

```bash
pipx install stout-pkg
```

## Usage

```bash
# Update formula index
stout update

# Search for packages
stout search nodejs

# Install packages
stout install wget curl

# Show package info
stout info python

# List installed packages
stout list
```

## Performance

| Operation | Homebrew | stout | Speedup |
|-----------|----------|-------|---------|
| --version | 500ms | 5ms | 100x |
| search | 2-5s | <50ms | 40-100x |
| info | 1-2s | <100ms | 10-20x |
| update | 10-60s | 1-3s | 10-20x |

## How It Works

This package is a thin Python wrapper that:
1. Downloads the native stout binary for your platform on first run
2. Caches it in your user cache directory
3. Forwards all commands to the native binary

## Supported Platforms

- macOS (Apple Silicon & Intel)
- Linux (x86_64 & ARM64)

## Links

- [GitHub Repository](https://github.com/neul-labs/stout)
- [Documentation](https://github.com/neul-labs/stout/blob/main/docs/USAGE.md)
- [Issue Tracker](https://github.com/neul-labs/stout/issues)

## License

MIT
