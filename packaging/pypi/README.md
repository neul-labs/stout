# stout

**A drop-in replacement for the Homebrew CLI that's 10-100x faster than brew.** stout is a fast, Rust-based package manager that eliminates Ruby entirely — instant search, info, and parallel installs, fully compatible with your existing `brew` setup.

[![PyPI](https://img.shields.io/pypi/v/stout-pkg)](https://pypi.org/project/stout-pkg/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://github.com/neul-labs/stout/blob/main/LICENSE)
[![Documentation](https://img.shields.io/badge/docs-docs.neullabs.com-blue)](https://docs.neullabs.com/stout)

**[Website](https://stout.neullabs.com) · [Documentation](https://docs.neullabs.com/stout) · [GitHub](https://github.com/neul-labs/stout)**

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

- [Website](https://stout.neullabs.com)
- [Documentation](https://docs.neullabs.com/stout)
- [GitHub Repository](https://github.com/neul-labs/stout)
- [Issue Tracker](https://github.com/neul-labs/stout/issues)

## Part of the Neul Labs toolchain

stout is part of the Neul Labs command-line & filesystem toolchain:

| Project | What it does |
|---------|--------------|
| [recurl](https://github.com/neul-labs/recurl) | curl that just works — drop-in replacement with automatic anti-bot bypass. |
| [rewget](https://github.com/neul-labs/rewget) | wget, but it works everywhere. |
| [stratafs](https://github.com/neul-labs/stratafs) | A semantic filesystem for AI-era search. |

Explore the full toolchain at [neullabs.com](https://www.neullabs.com).

## License

MIT
