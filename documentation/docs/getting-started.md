# Getting Started

This guide walks you through installing stout and getting started with package management.

---

## First Run

When you run stout for the first time on a system with existing Homebrew packages, stout will detect untracked packages in the Cellar and prompt you to import them:

```
Found 42 packages in Homebrew not tracked by stout.
Import them now? [Y/n]
```

Choose **Y** to import all packages, or **n** to skip. You can always import later with `stout import`.

---

## Quick Install (Recommended)

The fastest way to get started:

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

This script:

- Detects your OS and architecture
- Downloads the appropriate binary
- Installs to `~/.local/bin` (or `/usr/local/bin` with sudo)
- Sets up shell completions

---

## Package Managers

### npm

```bash
npm install -g stout-pkg
```

### pip

```bash
pip install stout-pkg
```

### gem

```bash
gem install stout-pkg
```

### Homebrew

```bash
brew install neul-labs/tap/stout
```

---

## Linux Packages

### Debian/Ubuntu

```bash
# Download the .deb package from releases
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout_amd64.deb
sudo dpkg -i stout_amd64.deb
```

### Fedora/RHEL

```bash
# Download the .rpm package from releases
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout.x86_64.rpm
sudo rpm -i stout.x86_64.rpm
```

### Arch Linux (AUR)

```bash
yay -S stout-bin
# or
paru -S stout-bin
```

### Nix

```bash
nix-env -iA nixpkgs.stout
```

---

## Build from Source

Requirements:

- Rust 1.75 or later
- Git

```bash
# Clone the repository
git clone https://github.com/neul-labs/stout.git
cd stout

# Build in release mode
cargo build --release

# Install the binary
cp target/release/stout ~/.local/bin/
```

---

## Shell Completions

Stout can generate shell completions for bash, zsh, and fish:

=== "Bash"

    ```bash
    stout completions bash > ~/.local/share/bash-completion/completions/stout
    ```

=== "Zsh"

    ```bash
    stout completions zsh > ~/.zfunc/_stout
    # Add to .zshrc: fpath=(~/.zfunc $fpath)
    ```

=== "Fish"

    ```bash
    stout completions fish > ~/.config/fish/completions/stout.fish
    ```

---

## Verify Installation

Check that stout is installed correctly:

```bash
stout --version
```

Run the health check:

```bash
stout doctor
```

---

## Upgrading

### Via install script

```bash
curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

### Via package manager

Use your package manager's update command (e.g., `npm update -g stout-pkg`).

### From source

```bash
cd stout
git pull
cargo build --release
cp target/release/stout ~/.local/bin/
```

---

## Uninstalling

### Binary installation

```bash
rm ~/.local/bin/stout
rm -rf ~/.stout  # Remove config and cache
```

### Package manager

Use your package manager's uninstall command.

---

## System Requirements

| Platform | Architecture | Status |
|----------|-------------|--------|
| macOS 12+ | x86_64 | Supported |
| macOS 12+ | arm64 (Apple Silicon) | Supported |
| Linux (glibc 2.17+) | x86_64 | Supported |
| Linux (glibc 2.17+) | arm64 | Supported |
| Windows | x86_64 | Planned |

---

## Next Steps

- [Quick Start](quickstart.md) - Learn the basics
- [Command Reference](commands.md) - See all available commands

---

## What `install.sh` Actually Does

The bootstrap script at `install.sh` in the repository root is the recommended
entry point. It honours a small set of environment variables:

| Variable | Purpose |
|----------|---------|
| `STOUT_INSTALL_DIR` | Override the install directory (default `~/.local/bin`, falls back to `/usr/local/bin` if writable) |
| `STOUT_VERSION` | Pin a specific release tag instead of `latest` |
| `STOUT_NO_MODIFY_PATH` | Set to `1` to skip PATH modification of shell rc files |

The script detects OS and architecture, downloads the matching tarball from the
GitHub releases page, verifies the published SHA256 checksum, and unpacks the
`stout` binary into the install directory.

```bash
# Install a specific release into /opt/bin without touching PATH
STOUT_INSTALL_DIR=/opt/bin \
STOUT_VERSION=v0.2.2 \
STOUT_NO_MODIFY_PATH=1 \
  curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
```

---

## Verifying a Downloaded Binary Manually

If you prefer to bypass `install.sh`, the release artifacts on
[github.com/neul-labs/stout/releases](https://github.com/neul-labs/stout/releases)
include a `SHA256SUMS` file. Verify before installing:

```bash
# Apple Silicon example
curl -LO https://github.com/neul-labs/stout/releases/latest/download/stout-aarch64-apple-darwin.tar.gz
curl -LO https://github.com/neul-labs/stout/releases/latest/download/SHA256SUMS
shasum -a 256 -c SHA256SUMS --ignore-missing
tar -xzf stout-aarch64-apple-darwin.tar.gz
install -m 0755 stout ~/.local/bin/stout
```

---

## First Run Behaviour

The first invocation creates `~/.stout/` containing the local state database
and a default `config.toml`. If stout detects packages already installed in
the Homebrew Cellar but not tracked in its state, it offers to import them in
bulk via the same code path as `stout import`. Decline the prompt and stout
will keep tracking only what you install through it; you can always run
`stout import` later.
