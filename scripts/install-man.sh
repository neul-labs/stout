#!/bin/bash
# Install stout man pages
# Usage: ./install-man.sh [--prefix /usr/local]

set -euo pipefail

PREFIX="${1:-/usr/local}"
MAN_DIR="${PREFIX}/share/man/man1"

# Check if we're in the stout repo
if [ ! -f "Cargo.toml" ]; then
    echo "Error: Run this script from the stout repository root"
    exit 1
fi

echo "Building stout with man page generation..."
STOUT_GEN_MAN=1 cargo build --release

# Find the man pages in the build output
OUT_DIR=$(find target/release/build -name "stout-*" -type d -path "*/out" 2>/dev/null | head -1)
MAN_SRC="${OUT_DIR}/man"

if [ ! -d "$MAN_SRC" ]; then
    echo "Error: Man pages not found. Build may have failed."
    exit 1
fi

echo "Installing man pages to ${MAN_DIR}..."
mkdir -p "$MAN_DIR"

for man_file in "$MAN_SRC"/*.1; do
    if [ -f "$man_file" ]; then
        name=$(basename "$man_file")
        echo "  Installing $name"
        install -m 644 "$man_file" "${MAN_DIR}/${name}"
    fi
done

echo ""
echo "Man pages installed successfully!"
echo "Try: man stout"
echo "     man stout-install"
echo "     man stout-search"
