#!/usr/bin/env bash
set -euo pipefail

# Cross-platform build and release script for stout
# Uses cargo-zigbuild for cross-compilation

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }
step() { echo -e "${BLUE}[STEP]${NC} $1"; }

# Get version from Cargo.toml
VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
TAG="v${VERSION}"

# Build targets
TARGETS=(
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    "aarch64-unknown-linux-gnu"
    "aarch64-apple-darwin"
    "x86_64-apple-darwin"
)

# Parse arguments
DRY_RUN=false
SKIP_BUILD=false
SKIP_UPLOAD=false
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
        --skip-build)
            SKIP_BUILD=true
            ;;
        --skip-upload)
            SKIP_UPLOAD=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Builds cross-platform binaries and creates a GitHub release."
            echo ""
            echo "Options:"
            echo "  --dry-run      Show what would be done without executing"
            echo "  --skip-build   Skip building binaries (use existing)"
            echo "  --skip-upload  Build only, don't create GitHub release"
            echo "  --help, -h     Show this help message"
            exit 0
            ;;
    esac
done

info "Stout Release Script"
info "Version: ${VERSION} (tag: ${TAG})"
echo ""

# Check prerequisites
step "Checking prerequisites..."

if ! command -v cargo-zigbuild &> /dev/null; then
    error "cargo-zigbuild is not installed. Install with: cargo install cargo-zigbuild"
    exit 1
fi

if ! command -v zig &> /dev/null; then
    error "zig is not installed. Install from: https://ziglang.org/"
    exit 1
fi

if ! command -v gh &> /dev/null; then
    error "GitHub CLI (gh) is not installed"
    exit 1
fi

if ! gh auth status &> /dev/null; then
    error "Not authenticated with GitHub. Run: gh auth login"
    exit 1
fi

# Ensure all targets are installed
step "Ensuring Rust targets are installed..."
for target in "${TARGETS[@]}"; do
    rustup target add "$target" 2>/dev/null || true
done

# Create dist directory
DIST_DIR="$ROOT_DIR/dist"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Build for each target
if ! $SKIP_BUILD; then
    step "Building binaries for ${#TARGETS[@]} targets..."
    echo ""

    for target in "${TARGETS[@]}"; do
        info "Building for ${target}..."

        if $DRY_RUN; then
            echo "  [dry-run] cargo zigbuild --release --target ${target}"
        else
            # Use cargo-zigbuild for cross-compilation
            STOUT_GEN_MAN=1 cargo zigbuild --release --target "$target"
        fi

        # Create tarball
        ARTIFACT_NAME="stout-${target}"
        BINARY_PATH="target/${target}/release/stout"

        if ! $DRY_RUN && [ -f "$BINARY_PATH" ]; then
            info "Packaging ${ARTIFACT_NAME}..."

            # Create temp dir for packaging
            TEMP_DIR=$(mktemp -d)
            cp "$BINARY_PATH" "$TEMP_DIR/stout"

            # Create tarball
            tar -czvf "$DIST_DIR/${ARTIFACT_NAME}.tar.gz" -C "$TEMP_DIR" stout

            # Create checksum
            cd "$DIST_DIR"
            sha256sum "${ARTIFACT_NAME}.tar.gz" > "${ARTIFACT_NAME}.tar.gz.sha256"
            cd "$ROOT_DIR"

            # Cleanup
            rm -rf "$TEMP_DIR"

            info "Created: ${ARTIFACT_NAME}.tar.gz"
        fi

        echo ""
    done
fi

# List built artifacts
step "Built artifacts:"
ls -la "$DIST_DIR"/*.tar.gz 2>/dev/null || echo "No artifacts found"
echo ""

# Create GitHub release
if ! $SKIP_UPLOAD; then
    step "Creating GitHub release ${TAG}..."

    if $DRY_RUN; then
        echo "[dry-run] git tag ${TAG}"
        echo "[dry-run] git push origin ${TAG}"
        echo "[dry-run] gh release create ${TAG} dist/* --generate-notes"
    else
        # Create and push tag if it doesn't exist on remote
        if ! git ls-remote --tags origin | grep -q "refs/tags/${TAG}"; then
            info "Creating tag ${TAG}..."
            git tag -a "${TAG}" -m "Release ${TAG}" 2>/dev/null || true

            info "Pushing tag to origin..."
            git push origin "${TAG}"
        else
            warn "Tag ${TAG} already exists on remote"
        fi

        # Check if release already exists
        if gh release view "${TAG}" &> /dev/null; then
            warn "Release ${TAG} already exists, uploading assets..."
            gh release upload "${TAG}" "$DIST_DIR"/*.tar.gz "$DIST_DIR"/*.sha256 --clobber
        else
            info "Creating release ${TAG}..."
            gh release create "${TAG}" \
                "$DIST_DIR"/*.tar.gz \
                "$DIST_DIR"/*.sha256 \
                --generate-notes \
                --title "Stout ${TAG}"
        fi

        info "Release created: https://github.com/neul-labs/stout/releases/tag/${TAG}"
    fi
fi

echo ""
info "Done!"
