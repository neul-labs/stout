{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, openssl
, sqlite
, installShellFiles
}:

rustPlatform.buildRustPackage rec {
  pname = "stout";
  version = "0.2.1";

  src = fetchFromGitHub {
    owner = "neul-labs";
    repo = "stout";
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
    installShellCompletion --cmd stout \
      --bash <($out/bin/stout completions bash) \
      --zsh <($out/bin/stout completions zsh) \
      --fish <($out/bin/stout completions fish)
  '';

  meta = with lib; {
    description = "Fast, Rust-based Homebrew-compatible package manager";
    homepage = "https://github.com/neul-labs/stout";
    license = licenses.mit;
    maintainers = with maintainers; [ ];
    mainProgram = "stout";
    platforms = platforms.unix;
  };
}
