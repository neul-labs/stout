{
  description = "brewx - Fast, Rust-based Homebrew-compatible package manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        brewx = pkgs.rustPlatform.buildRustPackage {
          pname = "brewx";
          version = "0.1.0";

          src = ../..;

          cargoLock = {
            lockFile = ../../Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            installShellFiles
          ];

          buildInputs = with pkgs; [
            openssl
            sqlite
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          postInstall = ''
            installShellCompletion --cmd brewx \
              --bash <($out/bin/brewx completions bash) \
              --zsh <($out/bin/brewx completions zsh) \
              --fish <($out/bin/brewx completions fish)
          '';

          meta = with pkgs.lib; {
            description = "Fast, Rust-based Homebrew-compatible package manager";
            homepage = "https://github.com/anthropics/brewx";
            license = licenses.mit;
            mainProgram = "brewx";
          };
        };
      in
      {
        packages = {
          default = brewx;
          brewx = brewx;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = brewx;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            sqlite
            cargo-watch
            cargo-audit
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
