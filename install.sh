#!/bin/bash
# stout installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/neul-labs/stout/main/install.sh | bash
#
# Environment variables:
#   STOUT_INSTALL_DIR - Installation directory (default: ~/.local/bin or /usr/local/bin)
#   STOUT_VERSION     - Specific version to install (default: latest)
#   STOUT_NO_MODIFY_PATH - Set to 1 to skip PATH modification

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# GitHub repository
REPO="neul-labs/stout"
BINARY_NAME="stout"

# Print functions
info() {
    printf "${BLUE}info${NC}: %s\n" "$1"
}

success() {
    printf "${GREEN}success${NC}: %s\n" "$1"
}

warn() {
    printf "${YELLOW}warning${NC}: %s\n" "$1"
}

error() {
    printf "${RED}error${NC}: %s\n" "$1" >&2
}

# Check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Detect OS
detect_os() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "darwin" ;;
        *)       error "Unsupported operating system: $os"; exit 1 ;;
    esac
}

# Detect architecture
detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)  echo "x86_64" ;;
        arm64|aarch64) echo "aarch64" ;;
        *)             error "Unsupported architecture: $arch"; exit 1 ;;
    esac
}

# Get the target triple for this platform
get_target() {
    local os="$1"
    local arch="$2"

    case "$os-$arch" in
        linux-x86_64)   echo "x86_64-unknown-linux-gnu" ;;
        linux-aarch64)  echo "aarch64-unknown-linux-gnu" ;;
        darwin-x86_64)  echo "x86_64-apple-darwin" ;;
        darwin-aarch64) echo "aarch64-apple-darwin" ;;
        *)              error "Unsupported platform: $os-$arch"; exit 1 ;;
    esac
}

# Get the latest release version from GitHub
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version

    if command_exists curl; then
        version=$(curl -fsSL "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    elif command_exists wget; then
        version=$(wget -qO- "$url" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if [ -z "$version" ]; then
        error "Failed to fetch latest version"
        exit 1
    fi

    echo "$version"
}

# Download a file
download() {
    local url="$1"
    local output="$2"

    info "Downloading from $url"

    if command_exists curl; then
        curl -fsSL "$url" -o "$output"
    elif command_exists wget; then
        wget -q "$url" -O "$output"
    else
        error "Neither curl nor wget found"
        exit 1
    fi
}

# Verify checksum
verify_checksum() {
    local file="$1"
    local expected="$2"
    local actual

    if command_exists sha256sum; then
        actual=$(sha256sum "$file" | cut -d' ' -f1)
    elif command_exists shasum; then
        actual=$(shasum -a 256 "$file" | cut -d' ' -f1)
    else
        warn "Cannot verify checksum: sha256sum/shasum not found"
        return 0
    fi

    if [ "$actual" != "$expected" ]; then
        error "Checksum verification failed"
        error "Expected: $expected"
        error "Actual:   $actual"
        return 1
    fi

    success "Checksum verified"
}

# Determine installation directory
get_install_dir() {
    if [ -n "${STOUT_INSTALL_DIR:-}" ]; then
        echo "$STOUT_INSTALL_DIR"
    elif [ -w "/usr/local/bin" ]; then
        echo "/usr/local/bin"
    else
        echo "$HOME/.local/bin"
    fi
}

# Add directory to PATH in shell config
add_to_path() {
    local dir="$1"
    local shell_config

    if [ "${STOUT_NO_MODIFY_PATH:-0}" = "1" ]; then
        return
    fi

    # Check if already in PATH
    if echo "$PATH" | tr ':' '\n' | grep -qx "$dir"; then
        return
    fi

    # Determine shell config file
    case "${SHELL:-/bin/bash}" in
        */zsh)  shell_config="$HOME/.zshrc" ;;
        */bash)
            if [ -f "$HOME/.bashrc" ]; then
                shell_config="$HOME/.bashrc"
            else
                shell_config="$HOME/.bash_profile"
            fi
            ;;
        */fish) shell_config="$HOME/.config/fish/config.fish" ;;
        *)      shell_config="$HOME/.profile" ;;
    esac

    # Check if the export line already exists
    if [ -f "$shell_config" ] && grep -q "export PATH=.*$dir" "$shell_config"; then
        return
    fi

    info "Adding $dir to PATH in $shell_config"

    case "${SHELL:-/bin/bash}" in
        */fish)
            mkdir -p "$(dirname "$shell_config")"
            echo "set -gx PATH $dir \$PATH" >> "$shell_config"
            ;;
        *)
            echo "" >> "$shell_config"
            echo "# Added by stout installer" >> "$shell_config"
            echo "export PATH=\"$dir:\$PATH\"" >> "$shell_config"
            ;;
    esac

    warn "Run 'source $shell_config' or restart your shell to update PATH"
}

# Install using cargo (fallback method)
install_with_cargo() {
    if ! command_exists cargo; then
        error "cargo not found. Please install Rust: https://rustup.rs"
        exit 1
    fi

    info "Installing stout using cargo..."

    if cargo install stout 2>/dev/null; then
        return 0
    fi

    # If crates.io install fails, try installing from git
    info "Trying to install from git repository..."
    if cargo install --git "https://github.com/${REPO}.git"; then
        return 0
    fi

    return 1
}

# Main installation function
main() {
    echo ""
    printf "${BOLD}${CYAN}stout${NC} installer\n"
    echo ""

    # Detect platform
    local os arch target
    os=$(detect_os)
    arch=$(detect_arch)
    target=$(get_target "$os" "$arch")

    info "Detected platform: $os-$arch ($target)"

    # Get version
    local version="${STOUT_VERSION:-}"
    local use_cargo_fallback=0

    if [ -z "$version" ]; then
        info "Fetching latest version..."
        if ! version=$(get_latest_version 2>/dev/null); then
            warn "Failed to fetch latest version from GitHub releases"
            use_cargo_fallback=1
        fi
    fi

    if [ "$use_cargo_fallback" = "0" ]; then
        info "Installing stout $version"

        # Set up URLs
        local base_url="https://github.com/${REPO}/releases/download/${version}"
        local archive_name="stout-${target}.tar.gz"
        local archive_url="${base_url}/${archive_name}"
        local checksum_url="${base_url}/${archive_name}.sha256"

        # Create temp directory
        local tmp_dir
        tmp_dir=$(mktemp -d)
        trap "rm -rf $tmp_dir" EXIT

        # Download archive and checksum
        local archive_path="${tmp_dir}/${archive_name}"
        local checksum_path="${tmp_dir}/${archive_name}.sha256"

        if ! download "$archive_url" "$archive_path" 2>/dev/null || \
           ! download "$checksum_url" "$checksum_path" 2>/dev/null; then
            warn "Failed to download pre-built binary, falling back to cargo install"
            use_cargo_fallback=1
        fi
    fi

    if [ "$use_cargo_fallback" = "1" ]; then
        if install_with_cargo; then
            echo ""
            success "stout installed successfully via cargo!"
            echo ""
            if command_exists stout; then
                printf "  ${BOLD}Version${NC}:  $(stout --version)\n"
            fi
            echo ""
            printf "  Run ${CYAN}stout update${NC} to download the formula index.\n"
            printf "  Run ${CYAN}stout --help${NC} to get started.\n"
            echo ""
            return 0
        else
            error "Failed to install stout"
            exit 1
        fi
    fi

    # Verify checksum
    local expected_checksum
    expected_checksum=$(cut -d' ' -f1 < "$checksum_path")
    verify_checksum "$archive_path" "$expected_checksum"

    # Extract archive
    info "Extracting archive..."
    tar -xzf "$archive_path" -C "$tmp_dir"

    # Determine installation directory
    local install_dir
    install_dir=$(get_install_dir)

    # Create installation directory if needed
    if [ ! -d "$install_dir" ]; then
        info "Creating $install_dir"
        mkdir -p "$install_dir"
    fi

    # Install binary
    local binary_path="${install_dir}/${BINARY_NAME}"
    info "Installing to $binary_path"

    if [ -f "$binary_path" ]; then
        warn "Replacing existing installation"
    fi

    mv "${tmp_dir}/${BINARY_NAME}" "$binary_path"
    chmod +x "$binary_path"

    # Add to PATH if needed
    add_to_path "$install_dir"

    # Verify installation
    echo ""
    if "$binary_path" --version >/dev/null 2>&1; then
        success "stout $version installed successfully!"
        echo ""
        printf "  ${BOLD}Location${NC}: $binary_path\n"
        printf "  ${BOLD}Version${NC}:  $("$binary_path" --version)\n"
        echo ""
        printf "  Run ${CYAN}stout update${NC} to download the formula index.\n"
        printf "  Run ${CYAN}stout --help${NC} to get started.\n"
    else
        error "Installation completed but stout failed to run"
        exit 1
    fi

    echo ""
}

# Run main
main "$@"
