# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "stout-pkg"
  spec.version       = "0.1.0"
  spec.authors       = ["Neul Labs"]
  spec.email         = ["hello@neul.com"]

  spec.summary       = "A fast, Rust-based Homebrew-compatible package manager"
  spec.description   = "stout is a drop-in replacement for the Homebrew CLI that's 10-100x faster for common operations like search, info, and update."
  spec.homepage      = "https://github.com/neul-labs/stout"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.6.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/neul-labs/stout"
  spec.metadata["changelog_uri"] = "https://github.com/neul-labs/stout/releases"
  spec.metadata["bug_tracker_uri"] = "https://github.com/neul-labs/stout/issues"

  spec.files = Dir["lib/**/*", "exe/*", "README.md", "LICENSE"]
  spec.bindir = "exe"
  spec.executables = ["stout"]
  spec.require_paths = ["lib"]

  spec.post_install_message = <<~MSG
    Thanks for installing stout!

    Run 'stout update' to download the formula index.
    Run 'stout --help' to get started.
  MSG
end
