{
  description = "stout - Fast, Rust-based Homebrew-compatible package manager";

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

        stout = pkgs.rustPlatform.buildRustPackage {
          pname = "stout";
          version = "0.2.1";

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

          STOUT_GEN_MAN = "1";

          postInstall = ''
            installShellCompletion --cmd stout \
              --bash <($out/bin/stout completions bash) \
              --zsh <($out/bin/stout completions zsh) \
              --fish <($out/bin/stout completions fish)

            # Install man pages
            manDir=$(find target -name "stout-*" -type d -path "*/out" | head -1)/man
            if [ -d "$manDir" ]; then
              installManPage $manDir/*.1
            fi
          '';

          meta = with pkgs.lib; {
            description = "Fast, Rust-based Homebrew-compatible package manager";
            homepage = "https://github.com/neul-labs/stout";
            license = licenses.mit;
            mainProgram = "stout";
          };
        };
      in
      {
        packages = {
          default = stout;
          stout = stout;
        };

        apps.default = flake-utils.lib.mkApp {
          drv = stout;
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
