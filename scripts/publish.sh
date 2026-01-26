#!/usr/bin/env bash
set -euo pipefail

# Publish all stout crates to crates.io in dependency order

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$ROOT_DIR"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Delay between publishes to allow crates.io indexing
PUBLISH_DELAY=15

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

publish_crate() {
    local crate=$1
    info "Publishing $crate..."

    if cargo publish -p "$crate" "$@"; then
        info "$crate published successfully"
        return 0
    else
        error "Failed to publish $crate"
        return 1
    fi
}

# Parse arguments
DRY_RUN=false
SKIP_DELAY=false
for arg in "$@"; do
    case $arg in
        --dry-run)
            DRY_RUN=true
            ;;
        --no-delay)
            SKIP_DELAY=true
            ;;
        --help|-h)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --dry-run    Run 'cargo publish --dry-run' instead of actual publish"
            echo "  --no-delay   Skip delay between publishes"
            echo "  --help, -h   Show this help message"
            exit 0
            ;;
    esac
done

PUBLISH_ARGS=""
if $DRY_RUN; then
    PUBLISH_ARGS="--dry-run"
    warn "Dry run mode - no packages will be published"
fi

# Crates in dependency order
CRATES=(
    # Layer 1 - no internal dependencies
    "stout-index"
    "stout-fetch"

    # Layer 2 - depends on layer 1
    "stout-resolve"
    "stout-state"

    # Layer 3 - depends on layers above
    "stout-install"
    "stout-cask"
    "stout-bundle"
    "stout-audit"

    # Layer 4 - depends on index, resolve, state
    "stout-mirror"

    # Layer 5 - main crate
    "stout"
)

info "Publishing ${#CRATES[@]} crates to crates.io"
echo ""

for i in "${!CRATES[@]}"; do
    crate="${CRATES[$i]}"
    count=$((i + 1))

    info "[$count/${#CRATES[@]}] Publishing $crate"

    if ! cargo publish -p "$crate" $PUBLISH_ARGS; then
        error "Failed to publish $crate"
        exit 1
    fi

    # Wait for crates.io to index the package (except for last crate or dry run)
    if ! $DRY_RUN && ! $SKIP_DELAY && [ $count -lt ${#CRATES[@]} ]; then
        info "Waiting ${PUBLISH_DELAY}s for crates.io to index..."
        sleep $PUBLISH_DELAY
    fi

    echo ""
done

info "All crates published successfully!"
