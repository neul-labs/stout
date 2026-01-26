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
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Delay between publishes to allow crates.io indexing
PUBLISH_DELAY=15

# Rate limit retry settings
RATE_LIMIT_RETRIES=3
RATE_LIMIT_WAIT=600  # 10 minutes

info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

skip() {
    echo -e "${BLUE}[SKIP]${NC} $1"
}

# Get the version of a crate from its Cargo.toml
get_crate_version() {
    local crate=$1
    local cargo_toml

    if [ "$crate" = "stout" ]; then
        cargo_toml="$ROOT_DIR/Cargo.toml"
    else
        cargo_toml="$ROOT_DIR/crates/$crate/Cargo.toml"
    fi

    # Check if version is inherited from workspace
    if grep -q 'version.workspace = true' "$cargo_toml"; then
        # Get version from workspace root
        grep '^version = ' "$ROOT_DIR/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
    else
        grep '^version = ' "$cargo_toml" | head -1 | sed 's/version = "\(.*\)"/\1/'
    fi
}

# Check if a specific version of a crate is already published
is_already_published() {
    local crate=$1
    local version=$2

    # Use cargo search to check if the exact version exists
    local published_version
    published_version=$(cargo search "$crate" --limit 1 2>/dev/null | grep "^$crate = " | sed 's/.*"\(.*\)".*/\1/' || echo "")

    if [ "$published_version" = "$version" ]; then
        return 0  # Already published
    else
        return 1  # Not published
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
            echo "Publishes all stout crates to crates.io in dependency order."
            echo "Already-published crates are automatically skipped."
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

published_count=0
skipped_count=0

for i in "${!CRATES[@]}"; do
    crate="${CRATES[$i]}"
    count=$((i + 1))
    version=$(get_crate_version "$crate")

    echo "[$count/${#CRATES[@]}] $crate v$version"

    # Check if already published (skip check for dry-run)
    if ! $DRY_RUN && is_already_published "$crate" "$version"; then
        skip "$crate v$version is already published, skipping"
        ((skipped_count++)) || true
        echo ""
        continue
    fi

    # Attempt to publish with rate limit retry
    attempt=1
    while [ $attempt -le $RATE_LIMIT_RETRIES ]; do
        output=$(cargo publish -p "$crate" $PUBLISH_ARGS 2>&1) && publish_status=0 || publish_status=$?

        if [ $publish_status -eq 0 ]; then
            info "$crate v$version published successfully"
            ((published_count++)) || true
            break
        fi

        # Check if it's a rate limit error
        if echo "$output" | grep -q "429 Too Many Requests"; then
            if [ $attempt -lt $RATE_LIMIT_RETRIES ]; then
                warn "Rate limited. Waiting ${RATE_LIMIT_WAIT}s before retry (attempt $attempt/$RATE_LIMIT_RETRIES)..."
                sleep $RATE_LIMIT_WAIT
                ((attempt++)) || true
            else
                error "Rate limit exceeded after $RATE_LIMIT_RETRIES retries"
                echo "$output"
                exit 1
            fi
        # Check if it's an "already exists" error (race condition)
        elif echo "$output" | grep -q "already exists"; then
            skip "$crate v$version was just published, skipping"
            ((skipped_count++)) || true
            break
        else
            error "Failed to publish $crate"
            echo "$output"
            exit 1
        fi
    done

    # Wait for crates.io to index the package (except for last crate or dry run)
    if ! $DRY_RUN && ! $SKIP_DELAY && [ $count -lt ${#CRATES[@]} ]; then
        info "Waiting ${PUBLISH_DELAY}s for crates.io to index..."
        sleep $PUBLISH_DELAY
    fi

    echo ""
done

info "Done! Published: $published_count, Skipped: $skipped_count"
