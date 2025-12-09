# frozen_string_literal: true

require_relative "stout_pkg/version"
require_relative "stout_pkg/installer"

module StoutPkg
  class Error < StandardError; end

  REPO = "neul-labs/stout"
  BINARY_NAME = "stout"

  class << self
    def binary_path
      Installer.ensure_binary
    end

    def run(*args)
      binary = binary_path
      exec(binary.to_s, *args)
    end
  end
end
