# RPM Packaging for stout

This directory contains documentation for building RPM packages (.rpm) for stout.

## Automated Builds

RPM packages are automatically built and published to GitHub Releases when a new version tag is pushed. The release workflow builds packages for:

- `x86_64`
- `aarch64`

## Manual Build

To build an .rpm package locally:

```bash
# Install cargo-generate-rpm
cargo install cargo-generate-rpm

# Build the release binary with man pages
STOUT_GEN_MAN=1 cargo build --release

# Generate shell completions
mkdir -p target/completions
./target/release/stout completions bash > target/completions/stout.bash
./target/release/stout completions zsh > target/completions/_stout
./target/release/stout completions fish > target/completions/stout.fish

# Build the .rpm package
cargo generate-rpm
```

The resulting `.rpm` file will be in `target/generate-rpm/`.

## Installation

### Fedora / RHEL / CentOS / Rocky Linux

```bash
# Download from GitHub releases
wget https://github.com/neul-labs/stout/releases/latest/download/stout-VERSION-1.x86_64.rpm

# Install with dnf
sudo dnf install ./stout-VERSION-1.x86_64.rpm

# Or with rpm directly
sudo rpm -i stout-VERSION-1.x86_64.rpm
```

### openSUSE

```bash
# Download from GitHub releases
wget https://github.com/neul-labs/stout/releases/latest/download/stout-VERSION-1.x86_64.rpm

# Install with zypper
sudo zypper install ./stout-VERSION-1.x86_64.rpm
```

## Package Contents

The .rpm package includes:
- `/usr/bin/stout` - The main binary
- `/usr/share/doc/stout/` - Documentation (README, LICENSE)
- `/usr/share/bash-completion/completions/stout` - Bash completions
- `/usr/share/fish/vendor_completions.d/stout.fish` - Fish completions
- `/usr/share/zsh/site-functions/_stout` - Zsh completions

## Configuration

The packaging configuration is in the root `Cargo.toml` under `[package.metadata.generate-rpm]`.

## COPR Repository (Optional)

For easier installation, you can set up a COPR repository:

1. Create an account at https://copr.fedorainfracloud.org/
2. Create a new project
3. Upload the .spec file or configure automatic builds from GitHub releases
4. Users can then install with:
   ```bash
   sudo dnf copr enable neul-labs/stout
   sudo dnf install stout
   ```
