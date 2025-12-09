#!/bin/bash
# Generate shell completions for stout
# This script is used during packaging to generate completion files

set -e

BINARY="${1:-./target/release/stout}"
OUTPUT_DIR="${2:-./target/completions}"

mkdir -p "$OUTPUT_DIR"

echo "Generating shell completions..."

"$BINARY" completions bash > "$OUTPUT_DIR/stout.bash"
"$BINARY" completions zsh > "$OUTPUT_DIR/_stout"
"$BINARY" completions fish > "$OUTPUT_DIR/stout.fish"

echo "Completions generated in $OUTPUT_DIR"
ls -la "$OUTPUT_DIR"
