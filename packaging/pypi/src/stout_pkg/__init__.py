"""
stout - A fast, Rust-based Homebrew-compatible package manager

This package provides a Python wrapper that downloads and runs the native stout binary.
"""

__version__ = "0.2.2"

import os
import platform
import stat
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
from pathlib import Path

REPO = "neul-labs/stout"
BINARY_NAME = "stout"


def get_platform():
    """Get the current platform identifier."""
    system = platform.system().lower()
    if system == "darwin":
        return "darwin"
    elif system == "linux":
        return "linux"
    else:
        raise RuntimeError(f"Unsupported platform: {system}")


def get_arch():
    """Get the current architecture identifier."""
    machine = platform.machine().lower()
    if machine in ("x86_64", "amd64"):
        return "x86_64"
    elif machine in ("arm64", "aarch64"):
        return "aarch64"
    else:
        raise RuntimeError(f"Unsupported architecture: {machine}")


def get_target(plat: str, arch: str) -> str:
    """Get the Rust target triple for this platform."""
    targets = {
        ("darwin", "x86_64"): "x86_64-apple-darwin",
        ("darwin", "aarch64"): "aarch64-apple-darwin",
        ("linux", "x86_64"): "x86_64-unknown-linux-gnu",
        ("linux", "aarch64"): "aarch64-unknown-linux-gnu",
    }
    target = targets.get((plat, arch))
    if not target:
        raise RuntimeError(f"Unsupported platform/arch: {plat}-{arch}")
    return target


def get_binary_dir() -> Path:
    """Get the directory where the binary should be stored."""
    # Store in user's cache directory
    if sys.platform == "darwin":
        cache_dir = Path.home() / "Library" / "Caches" / "stout-pkg"
    else:
        cache_dir = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache")) / "stout-pkg"
    cache_dir.mkdir(parents=True, exist_ok=True)
    return cache_dir


def get_binary_path() -> Path:
    """Get the path to the stout binary."""
    return get_binary_dir() / BINARY_NAME


def get_latest_version() -> str:
    """Fetch the latest release version from GitHub."""
    url = f"https://api.github.com/repos/{REPO}/releases/latest"
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": "stout-pypi-installer",
            "Accept": "application/vnd.github.v3+json",
        },
    )
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            import json
            data = json.loads(response.read().decode())
            return data["tag_name"]
    except Exception:
        return f"v{__version__}"


def download_binary(version: str = None) -> Path:
    """Download the stout binary for this platform."""
    if version is None:
        version = get_latest_version()

    plat = get_platform()
    arch = get_arch()
    target = get_target(plat, arch)

    archive_name = f"stout-{target}.tar.gz"
    download_url = f"https://github.com/{REPO}/releases/download/{version}/{archive_name}"

    binary_path = get_binary_path()
    version_file = get_binary_dir() / "version"

    # Check if we already have this version
    if binary_path.exists() and version_file.exists():
        installed_version = version_file.read_text().strip()
        if installed_version == version:
            return binary_path

    print(f"Downloading stout {version} for {plat}-{arch}...")

    with tempfile.TemporaryDirectory() as tmp_dir:
        tmp_path = Path(tmp_dir)
        archive_path = tmp_path / archive_name

        # Download
        req = urllib.request.Request(
            download_url,
            headers={"User-Agent": "stout-pypi-installer"},
        )
        with urllib.request.urlopen(req, timeout=60) as response:
            with open(archive_path, "wb") as f:
                f.write(response.read())

        # Extract
        with tarfile.open(archive_path, "r:gz") as tar:
            tar.extractall(tmp_path)

        # Move binary
        extracted_binary = tmp_path / BINARY_NAME
        if binary_path.exists():
            binary_path.unlink()
        extracted_binary.rename(binary_path)

        # Make executable
        binary_path.chmod(binary_path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

        # Write version file
        version_file.write_text(version)

    print(f"stout installed to {binary_path}")
    return binary_path


def ensure_binary() -> Path:
    """Ensure the binary is downloaded and return its path."""
    binary_path = get_binary_path()
    if not binary_path.exists():
        return download_binary()
    return binary_path


def main():
    """Main entry point - runs stout with all provided arguments."""
    try:
        binary_path = ensure_binary()
        result = subprocess.run([str(binary_path)] + sys.argv[1:])
        sys.exit(result.returncode)
    except KeyboardInterrupt:
        sys.exit(130)
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        print("You can install stout manually from: https://github.com/neul-labs/stout/releases", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
