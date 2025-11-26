# Nix Distribution

This directory contains files for distributing brewx via Nix.

## Installation Methods

### Using Nix Flakes (Recommended)

```bash
# Run without installing
nix run github:anthropics/brewx

# Install to profile
nix profile install github:anthropics/brewx

# Add to flake.nix inputs
{
  inputs.brewx.url = "github:anthropics/brewx";
}
```

### Using nix-env (Traditional)

```bash
# Clone the repository
git clone https://github.com/anthropics/brewx.git
cd brewx

# Install
nix-env -f packaging/nix/default.nix -i
```

### In NixOS Configuration

```nix
# configuration.nix
{ pkgs, ... }:

let
  brewx = pkgs.callPackage (builtins.fetchurl {
    url = "https://raw.githubusercontent.com/anthropics/brewx/main/packaging/nix/default.nix";
    sha256 = "PLACEHOLDER";
  }) {};
in
{
  environment.systemPackages = [ brewx ];
}
```

### In Home Manager

```nix
# home.nix
{ pkgs, ... }:

let
  brewx = pkgs.callPackage ./brewx.nix {};
in
{
  home.packages = [ brewx ];
}
```

## Development Shell

The flake includes a development shell with all build dependencies:

```bash
# Enter dev shell
nix develop

# Or with direnv
echo "use flake" > .envrc
direnv allow
```

## Building Locally

```bash
# Build the package
nix build .#brewx

# Run directly
nix run .#brewx -- --help

# Check flake
nix flake check
```

## Updating the Package

When releasing a new version:

1. Update the version in `default.nix` and `flake.nix`
2. Update the source hash:

```bash
# Get the hash for a specific version
nix-prefetch-github anthropics brewx --rev v0.1.0
```

3. Update `flake.lock`:

```bash
nix flake update
```

## Submitting to nixpkgs

To add brewx to the official nixpkgs repository:

1. Fork https://github.com/NixOS/nixpkgs
2. Add the package to `pkgs/by-name/br/brewx/package.nix`:

```nix
{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, openssl
, sqlite
, installShellFiles
}:

rustPlatform.buildRustPackage rec {
  pname = "brewx";
  version = "0.1.0";

  src = fetchFromGitHub {
    owner = "anthropics";
    repo = "brewx";
    rev = "v${version}";
    hash = "sha256-PLACEHOLDER";
  };

  cargoHash = "sha256-PLACEHOLDER";

  nativeBuildInputs = [
    pkg-config
    installShellFiles
  ];

  buildInputs = [
    openssl
    sqlite
  ];

  postInstall = ''
    installShellCompletion --cmd brewx \
      --bash <($out/bin/brewx completions bash) \
      --zsh <($out/bin/brewx completions zsh) \
      --fish <($out/bin/brewx completions fish)
  '';

  meta = with lib; {
    description = "Fast, Rust-based Homebrew-compatible package manager";
    homepage = "https://github.com/anthropics/brewx";
    changelog = "https://github.com/anthropics/brewx/releases/tag/v${version}";
    license = licenses.mit;
    maintainers = with maintainers; [ /* your nixpkgs maintainer name */ ];
    mainProgram = "brewx";
    platforms = platforms.unix;
  };
}
```

3. Test the build:

```bash
nix-build -A brewx
```

4. Submit a pull request to nixpkgs

## Files

- `default.nix` - Standard Nix package expression
- `flake.nix` - Nix flakes configuration with dev shell
- `README.md` - This file
