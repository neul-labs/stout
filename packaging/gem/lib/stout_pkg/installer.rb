# frozen_string_literal: true

require "fileutils"
require "json"
require "net/http"
require "open-uri"
require "rbconfig"
require "rubygems/package"
require "zlib"

module StoutPkg
  module Installer
    REPO = "neul-labs/stout"
    BINARY_NAME = "stout"

    class << self
      def platform
        case RbConfig::CONFIG["host_os"]
        when /darwin/i
          "darwin"
        when /linux/i
          "linux"
        else
          raise Error, "Unsupported platform: #{RbConfig::CONFIG['host_os']}"
        end
      end

      def arch
        case RbConfig::CONFIG["host_cpu"]
        when /x86_64|amd64/i
          "x86_64"
        when /arm64|aarch64/i
          "aarch64"
        else
          raise Error, "Unsupported architecture: #{RbConfig::CONFIG['host_cpu']}"
        end
      end

      def target
        targets = {
          %w[darwin x86_64] => "x86_64-apple-darwin",
          %w[darwin aarch64] => "aarch64-apple-darwin",
          %w[linux x86_64] => "x86_64-unknown-linux-gnu",
          %w[linux aarch64] => "aarch64-unknown-linux-gnu"
        }
        key = [platform, arch]
        targets[key] || raise(Error, "Unsupported platform/arch: #{key.join('-')}")
      end

      def cache_dir
        dir = if platform == "darwin"
                File.join(Dir.home, "Library", "Caches", "stout-pkg")
              else
                File.join(ENV.fetch("XDG_CACHE_HOME", File.join(Dir.home, ".cache")), "stout-pkg")
              end
        FileUtils.mkdir_p(dir)
        dir
      end

      def binary_path
        File.join(cache_dir, BINARY_NAME)
      end

      def version_file
        File.join(cache_dir, "version")
      end

      def latest_version
        uri = URI("https://api.github.com/repos/#{REPO}/releases/latest")
        http = Net::HTTP.new(uri.host, uri.port)
        http.use_ssl = true
        http.open_timeout = 10
        http.read_timeout = 10

        request = Net::HTTP::Get.new(uri)
        request["User-Agent"] = "stout-gem-installer"
        request["Accept"] = "application/vnd.github.v3+json"

        response = http.request(request)
        data = JSON.parse(response.body)
        data["tag_name"]
      rescue StandardError
        "v#{VERSION}"
      end

      def download_binary(version = nil)
        version ||= latest_version
        archive_name = "stout-#{target}.tar.gz"
        download_url = "https://github.com/#{REPO}/releases/download/#{version}/#{archive_name}"

        # Check if we already have this version
        if File.exist?(binary_path) && File.exist?(version_file)
          installed = File.read(version_file).strip
          return binary_path if installed == version
        end

        puts "Downloading stout #{version} for #{platform}-#{arch}..."

        Dir.mktmpdir do |tmp_dir|
          archive_path = File.join(tmp_dir, archive_name)

          # Download
          URI.open(download_url, "User-Agent" => "stout-gem-installer") do |remote|
            File.open(archive_path, "wb") do |local|
              local.write(remote.read)
            end
          end

          # Extract tar.gz
          Zlib::GzipReader.open(archive_path) do |gz|
            Gem::Package::TarReader.new(gz) do |tar|
              tar.each do |entry|
                next unless entry.file? && entry.full_name == BINARY_NAME

                File.open(binary_path, "wb") do |f|
                  f.write(entry.read)
                end
                break
              end
            end
          end

          # Make executable
          File.chmod(0o755, binary_path)

          # Write version file
          File.write(version_file, version)
        end

        puts "stout installed to #{binary_path}"
        binary_path
      end

      def ensure_binary
        return binary_path if File.exist?(binary_path)

        download_binary
      end
    end
  end
end
