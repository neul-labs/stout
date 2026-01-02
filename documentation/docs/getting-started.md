# Installation

This guide covers all the ways to install Stout on your system.

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
