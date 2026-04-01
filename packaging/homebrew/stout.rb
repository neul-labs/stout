# typed: false
# frozen_string_literal: true

# Homebrew formula for stout
# To install: brew install neul-labs/tap/stout
# Or add tap: brew tap neul-labs/tap && brew install stout

class Stout < Formula
  desc "Fast, Rust-based Homebrew-compatible package manager"
  homepage "https://github.com/neul-labs/stout"
  version "0.2.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/neul-labs/stout/releases/download/v#{version}/stout-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/stout/releases/download/v#{version}/stout-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_X86_64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/neul-labs/stout/releases/download/v#{version}/stout-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/neul-labs/stout/releases/download/v#{version}/stout-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "stout"

    # Generate and install shell completions
    generate_completions_from_executable(bin/"stout", "completions")

    # Install man pages if present in the release archive
    man1.install Dir["man/*.1"] if Dir.exist?("man")
  end

  test do
    assert_match "stout #{version}", shell_output("#{bin}/stout --version")
    assert_match "formulas", shell_output("#{bin}/stout doctor 2>&1", 1)
  end
end
