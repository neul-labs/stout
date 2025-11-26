# typed: false
# frozen_string_literal: true

# Homebrew formula for brewx
# To install: brew install anthropics/tap/brewx
# Or add tap: brew tap anthropics/tap && brew install brewx

class Brewx < Formula
  desc "Fast, Rust-based Homebrew-compatible package manager"
  homepage "https://github.com/anthropics/brewx"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/anthropics/brewx/releases/download/v#{version}/brewx-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"
    end
    on_intel do
      url "https://github.com/anthropics/brewx/releases/download/v#{version}/brewx-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_X86_64"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/anthropics/brewx/releases/download/v#{version}/brewx-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"
    end
    on_intel do
      url "https://github.com/anthropics/brewx/releases/download/v#{version}/brewx-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"
    end
  end

  def install
    bin.install "brewx"

    # Generate and install shell completions
    generate_completions_from_executable(bin/"brewx", "completions")
  end

  test do
    assert_match "brewx #{version}", shell_output("#{bin}/brewx --version")
    assert_match "formulas", shell_output("#{bin}/brewx doctor 2>&1", 1)
  end
end
