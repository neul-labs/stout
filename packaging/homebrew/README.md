# Homebrew Distribution

This directory contains files for distributing brewx via Homebrew.

## Overview

brewx can be distributed via Homebrew in two ways:

1. **Custom Tap** (recommended for initial release): `brew install anthropics/tap/brewx`
2. **homebrew-core** (for wider adoption): `brew install brewx`

## Setting Up a Homebrew Tap

### 1. Create the Tap Repository

Create a new GitHub repository named `homebrew-tap` under the `anthropics` organization:

```bash
# Repository: github.com/anthropics/homebrew-tap
```

### 2. Repository Structure

```
homebrew-tap/
├── Formula/
│   └── brewx.rb      # Copy from this directory
└── README.md
```

### 3. Copy the Formula

After creating a release, update and copy the formula:

```bash
# Update SHA256 hashes for the new release
./update-formula.sh 0.1.0

# Copy to tap repository
cp brewx.rb /path/to/homebrew-tap/Formula/
```

### 4. Users Install Via

```bash
# Add tap (one-time)
brew tap anthropics/tap

# Install
brew install brewx

# Or in one command
brew install anthropics/tap/brewx
```

## Release Workflow Integration

The release workflow automatically:
1. Builds binaries for all platforms
2. Creates SHA256 checksums
3. Uploads to GitHub releases

After a release, run `update-formula.sh` to fetch the new checksums and update the formula.

### Automating Formula Updates

Add this job to `.github/workflows/release.yml` to auto-update the tap:

```yaml
update-homebrew:
  needs: release
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4

    - name: Update Homebrew Formula
      env:
        TAP_GITHUB_TOKEN: ${{ secrets.TAP_GITHUB_TOKEN }}
      run: |
        VERSION="${GITHUB_REF#refs/tags/v}"
        ./packaging/homebrew/update-formula.sh "$VERSION"

        # Clone tap repo and update
        git clone https://x-access-token:${TAP_GITHUB_TOKEN}@github.com/anthropics/homebrew-tap.git
        cp packaging/homebrew/brewx.rb homebrew-tap/Formula/
        cd homebrew-tap
        git config user.name "GitHub Actions"
        git config user.email "actions@github.com"
        git add Formula/brewx.rb
        git commit -m "Update brewx to $VERSION"
        git push
```

## Formula Details

### brewx.rb

The formula:
- Supports macOS (ARM and Intel) and Linux (x86_64 and ARM64)
- Downloads pre-built binaries from GitHub releases
- Verifies SHA256 checksums
- Installs shell completions automatically
- Includes a test block for `brew test brewx`

### Updating the Formula

```bash
# After creating a new release
./update-formula.sh <version>

# Example
./update-formula.sh 0.2.0
```

The script will:
1. Fetch SHA256 checksums from the release
2. Update the formula with new version and hashes
3. Output next steps

## Testing Locally

```bash
# Install from local formula
brew install --build-from-source ./brewx.rb

# Run tests
brew test brewx

# Audit the formula
brew audit --strict brewx
```

## Submitting to homebrew-core

Once brewx has gained adoption, consider submitting to homebrew-core:

1. Ensure the formula passes `brew audit --strict --new-formula brewx`
2. Fork homebrew/homebrew-core
3. Add formula to `Formula/b/brewx.rb`
4. Submit a pull request
5. Follow the review process

Requirements for homebrew-core:
- Must have a notable user base
- Must have multiple contributors
- Must have documentation
- Must pass all audits
