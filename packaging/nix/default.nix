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

  cargoLock = {
    lockFile = "${src}/Cargo.lock";
  };

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
    license = licenses.mit;
    maintainers = with maintainers; [ ];
    mainProgram = "brewx";
    platforms = platforms.unix;
  };
}
