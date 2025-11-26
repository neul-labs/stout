#!/bin/bash
# Script to update the Homebrew formula with correct SHA256 hashes
# Usage: ./update-formula.sh <version>
# Example: ./update-formula.sh 0.1.0

set -euo pipefail

VERSION="${1:-}"

if [ -z "$VERSION" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.0"
    exit 1
fi

# Remove 'v' prefix if present
VERSION="${VERSION#v}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FORMULA_FILE="${SCRIPT_DIR}/brewx.rb"
BASE_URL="https://github.com/anthropics/brewx/releases/download/v${VERSION}"

echo "Updating formula for version ${VERSION}..."

# Function to get SHA256 from GitHub release
get_sha256() {
    local artifact="$1"
    local url="${BASE_URL}/${artifact}.sha256"
    curl -fsSL "$url" | cut -d' ' -f1
}

# Get SHA256 hashes for all platforms
echo "Fetching SHA256 hashes..."
SHA_MACOS_ARM64=$(get_sha256 "brewx-aarch64-apple-darwin.tar.gz")
SHA_MACOS_X86_64=$(get_sha256 "brewx-x86_64-apple-darwin.tar.gz")
SHA_LINUX_ARM64=$(get_sha256 "brewx-aarch64-unknown-linux-gnu.tar.gz")
SHA_LINUX_X86_64=$(get_sha256 "brewx-x86_64-unknown-linux-gnu.tar.gz")

echo "  macOS ARM64:  ${SHA_MACOS_ARM64}"
echo "  macOS x86_64: ${SHA_MACOS_X86_64}"
echo "  Linux ARM64:  ${SHA_LINUX_ARM64}"
echo "  Linux x86_64: ${SHA_LINUX_X86_64}"

# Update the formula file
echo "Updating formula..."

sed -i.bak \
    -e "s/version \".*\"/version \"${VERSION}\"/" \
    -e "s/PLACEHOLDER_SHA256_MACOS_ARM64/${SHA_MACOS_ARM64}/" \
    -e "s/PLACEHOLDER_SHA256_MACOS_X86_64/${SHA_MACOS_X86_64}/" \
    -e "s/PLACEHOLDER_SHA256_LINUX_ARM64/${SHA_LINUX_ARM64}/" \
    -e "s/PLACEHOLDER_SHA256_LINUX_X86_64/${SHA_LINUX_X86_64}/" \
    "$FORMULA_FILE"

# Also update existing hashes if they're not placeholders
sed -i.bak \
    -e "/aarch64-apple-darwin/,/sha256/s/sha256 \"[a-f0-9]\{64\}\"/sha256 \"${SHA_MACOS_ARM64}\"/" \
    -e "/x86_64-apple-darwin/,/sha256/s/sha256 \"[a-f0-9]\{64\}\"/sha256 \"${SHA_MACOS_X86_64}\"/" \
    -e "/aarch64-unknown-linux-gnu/,/sha256/s/sha256 \"[a-f0-9]\{64\}\"/sha256 \"${SHA_LINUX_ARM64}\"/" \
    -e "/x86_64-unknown-linux-gnu/,/sha256/s/sha256 \"[a-f0-9]\{64\}\"/sha256 \"${SHA_LINUX_X86_64}\"/" \
    "$FORMULA_FILE"

rm -f "${FORMULA_FILE}.bak"

echo "Formula updated successfully!"
echo ""
echo "Next steps:"
echo "1. Test locally: brew install --build-from-source ${FORMULA_FILE}"
echo "2. Push to homebrew-tap repository"
echo "3. Or submit PR to homebrew-core"
