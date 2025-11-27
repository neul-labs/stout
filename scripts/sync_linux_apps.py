#!/usr/bin/env python3
"""
Sync Linux applications metadata from various sources.

This script creates a unified index of Linux applications from:
- AppImageHub (AppImage format)
- Flathub (Flatpak format)
- Snapcraft (Snap format - optional)

The resulting SQLite database and compressed JSON files follow the same
pattern as the Homebrew formula and cask indexes.
"""

import argparse
import hashlib
import json
import logging
import os
import sqlite3
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional
from urllib.request import urlopen, Request
from urllib.error import HTTPError, URLError

logging.basicConfig(level=logging.INFO, format='%(levelname)s: %(message)s')
logger = logging.getLogger(__name__)

# API endpoints
APPIMAGEHUB_API = "https://appimage.github.io/feed.json"
FLATHUB_API = "https://flathub.org/api/v2/appstream"

@dataclass
class LinuxApp:
    """Represents a Linux application from any source."""
    token: str  # Unique identifier (lowercase, hyphenated)
    name: str
    version: str
    desc: Optional[str] = None
    homepage: Optional[str] = None
    source: str = ""  # "appimage", "flatpak", "snap"
    download_url: Optional[str] = None
    sha256: Optional[str] = None
    icon_url: Optional[str] = None
    categories: list = field(default_factory=list)
    license: Optional[str] = None
    # Source-specific identifiers
    flatpak_id: Optional[str] = None
    snap_name: Optional[str] = None

def fetch_json(url: str, timeout: int = 30) -> dict:
    """Fetch JSON from a URL."""
    request = Request(url, headers={'User-Agent': 'brewx/0.1'})
    try:
        with urlopen(request, timeout=timeout) as response:
            return json.loads(response.read().decode('utf-8'))
    except (HTTPError, URLError) as e:
        logger.error(f"Failed to fetch {url}: {e}")
        return {}

def tokenize(name: str) -> str:
    """Convert app name to a token (lowercase, hyphenated)."""
    # Replace spaces and underscores with hyphens, lowercase
    token = name.lower().replace(' ', '-').replace('_', '-')
    # Remove special characters
    token = ''.join(c for c in token if c.isalnum() or c == '-')
    # Remove duplicate hyphens
    while '--' in token:
        token = token.replace('--', '-')
    return token.strip('-')

def fetch_appimages() -> list[LinuxApp]:
    """Fetch AppImage catalog from AppImageHub."""
    logger.info("Fetching AppImageHub catalog...")

    data = fetch_json(APPIMAGEHUB_API)
    if not data or 'items' not in data:
        logger.warning("No AppImage data available")
        return []

    apps = []
    for item in data.get('items', []):
        try:
            name = item.get('name', '')
            if not name:
                continue

            # Get the most recent release
            links = item.get('links', [])
            download_url = None
            for link in links:
                if link.get('type') == 'Download':
                    download_url = link.get('url')
                    break

            app = LinuxApp(
                token=tokenize(name),
                name=name,
                version=item.get('version', 'latest'),
                desc=item.get('description', ''),
                homepage=item.get('authors', [{}])[0].get('url') if item.get('authors') else None,
                source='appimage',
                download_url=download_url,
                icon_url=item.get('icons', [None])[0] if item.get('icons') else None,
                categories=item.get('categories', []),
                license=item.get('license'),
            )
            apps.append(app)
        except Exception as e:
            logger.debug(f"Failed to parse AppImage entry: {e}")
            continue

    logger.info(f"Found {len(apps)} AppImages")
    return apps

def fetch_flatpaks() -> list[LinuxApp]:
    """Fetch Flatpak catalog from Flathub."""
    logger.info("Fetching Flathub catalog...")

    apps = []
    seen_ids = set()

    try:
        # Fetch the main appstream data
        data = fetch_json(FLATHUB_API)
        if not data:
            logger.warning("No Flathub data available")
            return []

        # Handle both list and dict formats from the API
        items = []
        if isinstance(data, list):
            items = data
        elif isinstance(data, dict):
            items = list(data.values()) if data else []

        for app_data in items:
            try:
                # Get app_id from the data
                app_id = app_data.get('id', app_data.get('flatpakAppId', ''))
                if not app_id or app_id in seen_ids:
                    continue
                seen_ids.add(app_id)

                name = app_data.get('name', app_id.split('.')[-1])

                # Safely extract categories, filtering None values
                categories = app_data.get('categories', []) or []
                categories = [c for c in categories if c is not None]

                app = LinuxApp(
                    token=tokenize(name),
                    name=name,
                    version=app_data.get('version', 'latest'),
                    desc=app_data.get('summary', ''),
                    homepage=app_data.get('url', app_data.get('homepage', '')),
                    source='flatpak',
                    flatpak_id=app_id,
                    icon_url=app_data.get('icon'),
                    categories=categories,
                    license=app_data.get('license'),
                )
                apps.append(app)
            except Exception as e:
                logger.debug(f"Failed to parse Flatpak entry: {e}")
                continue

    except Exception as e:
        logger.warning(f"Failed to fetch Flathub catalog: {e}")

    logger.info(f"Found {len(apps)} Flatpaks")
    return apps

def create_database(apps: list[LinuxApp], db_path: Path) -> None:
    """Create SQLite database with Linux apps."""
    logger.info(f"Creating database at {db_path}...")

    db_path.parent.mkdir(parents=True, exist_ok=True)
    if db_path.exists():
        db_path.unlink()

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # Create schema
    cursor.executescript("""
        CREATE TABLE linux_apps (
            token TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            desc TEXT,
            homepage TEXT,
            source TEXT NOT NULL,
            download_url TEXT,
            sha256 TEXT,
            icon_url TEXT,
            categories TEXT,
            license TEXT,
            flatpak_id TEXT,
            snap_name TEXT,
            json_hash TEXT
        );

        CREATE INDEX idx_linux_apps_source ON linux_apps(source);
        CREATE INDEX idx_linux_apps_name ON linux_apps(name);

        -- Full-text search
        CREATE VIRTUAL TABLE linux_apps_fts USING fts5(
            token,
            name,
            desc,
            categories,
            content='linux_apps',
            content_rowid='rowid'
        );

        -- Triggers for FTS sync
        CREATE TRIGGER linux_apps_ai AFTER INSERT ON linux_apps BEGIN
            INSERT INTO linux_apps_fts(rowid, token, name, desc, categories)
            VALUES (new.rowid, new.token, new.name, new.desc, new.categories);
        END;

        CREATE TRIGGER linux_apps_ad AFTER DELETE ON linux_apps BEGIN
            INSERT INTO linux_apps_fts(linux_apps_fts, rowid, token, name, desc, categories)
            VALUES ('delete', old.rowid, old.token, old.name, old.desc, old.categories);
        END;

        CREATE TRIGGER linux_apps_au AFTER UPDATE ON linux_apps BEGIN
            INSERT INTO linux_apps_fts(linux_apps_fts, rowid, token, name, desc, categories)
            VALUES ('delete', old.rowid, old.token, old.name, old.desc, old.categories);
            INSERT INTO linux_apps_fts(rowid, token, name, desc, categories)
            VALUES (new.rowid, new.token, new.name, new.desc, new.categories);
        END;

        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );
    """)

    # Insert apps
    for app in apps:
        # Filter out None values from categories before joining
        clean_categories = [c for c in (app.categories or []) if c is not None]
        categories_str = ','.join(clean_categories)

        # Calculate JSON hash for delta updates
        app_json = json.dumps({
            'token': app.token,
            'name': app.name,
            'version': app.version,
            'desc': app.desc,
            'source': app.source,
        }, sort_keys=True)
        json_hash = hashlib.sha256(app_json.encode()).hexdigest()[:16]

        cursor.execute("""
            INSERT OR REPLACE INTO linux_apps
            (token, name, version, desc, homepage, source, download_url, sha256,
             icon_url, categories, license, flatpak_id, snap_name, json_hash)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            app.token,
            app.name,
            app.version,
            app.desc,
            app.homepage,
            app.source,
            app.download_url,
            app.sha256,
            app.icon_url,
            categories_str,
            app.license,
            app.flatpak_id,
            app.snap_name,
            json_hash,
        ))

    # Insert metadata
    cursor.execute("INSERT INTO meta (key, value) VALUES ('app_count', ?)", (len(apps),))
    cursor.execute("INSERT INTO meta (key, value) VALUES ('version', '1')")

    conn.commit()
    conn.close()

    logger.info(f"Inserted {len(apps)} Linux apps into database")

def write_compressed_json(apps: list[LinuxApp], output_dir: Path) -> None:
    """Write individual compressed JSON files for each app."""
    import zstandard as zstd

    apps_dir = output_dir / "linux-apps" / "data"
    apps_dir.mkdir(parents=True, exist_ok=True)

    compressor = zstd.ZstdCompressor(level=19)

    for app in apps:
        app_data = {
            'token': app.token,
            'name': app.name,
            'version': app.version,
            'desc': app.desc,
            'homepage': app.homepage,
            'source': app.source,
            'download_url': app.download_url,
            'sha256': app.sha256,
            'icon_url': app.icon_url,
            'categories': app.categories,
            'license': app.license,
            'flatpak_id': app.flatpak_id,
            'snap_name': app.snap_name,
        }

        # Use first letter subdirectory for organization
        first_letter = app.token[0].lower() if app.token else '_'
        subdir = apps_dir / first_letter
        subdir.mkdir(exist_ok=True)

        json_bytes = json.dumps(app_data, separators=(',', ':')).encode('utf-8')
        compressed = compressor.compress(json_bytes)

        output_file = subdir / f"{app.token}.json.zst"
        output_file.write_bytes(compressed)

    logger.info(f"Wrote {len(apps)} compressed JSON files")

def main():
    import zstandard as zstd
    from datetime import datetime

    parser = argparse.ArgumentParser(description="Sync Linux apps metadata")
    parser.add_argument('--output', '-o', default='.',
                       help='Output directory (default: current directory)')
    parser.add_argument('--sources', '-s', nargs='+',
                       default=['appimage', 'flatpak'],
                       choices=['appimage', 'flatpak', 'snap'],
                       help='Sources to sync (default: appimage flatpak)')
    parser.add_argument('--no-compress', action='store_true',
                       help='Skip writing compressed JSON files')
    parser.add_argument('--verbose', '-v', action='store_true',
                       help='Enable verbose logging')
    args = parser.parse_args()

    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)

    output_dir = Path(args.output)
    output_dir.mkdir(parents=True, exist_ok=True)

    # Create linux-apps directory structure
    linux_apps_dir = output_dir / "linux-apps"
    linux_apps_dir.mkdir(exist_ok=True)
    linux_apps_data_dir = linux_apps_dir / "data"
    linux_apps_data_dir.mkdir(exist_ok=True)

    all_apps = []

    # Fetch from each source
    if 'appimage' in args.sources:
        all_apps.extend(fetch_appimages())

    if 'flatpak' in args.sources:
        all_apps.extend(fetch_flatpaks())

    # TODO: Add snap support
    # if 'snap' in args.sources:
    #     all_apps.extend(fetch_snaps())

    if not all_apps:
        logger.error("No apps found from any source")
        sys.exit(1)

    # Deduplicate by token (prefer flatpak over appimage)
    seen_tokens = {}
    for app in all_apps:
        if app.token not in seen_tokens:
            seen_tokens[app.token] = app
        elif app.source == 'flatpak':
            # Prefer flatpak as it's more standardized
            seen_tokens[app.token] = app

    unique_apps = list(seen_tokens.values())
    logger.info(f"Total unique apps: {len(unique_apps)}")

    # Create database in linux-apps/
    db_path = linux_apps_dir / "index.db"
    create_database(unique_apps, db_path)

    # Compress database
    logger.info("Compressing database...")
    db_bytes = db_path.read_bytes()
    compressor = zstd.ZstdCompressor(level=19)
    compressed_db = compressor.compress(db_bytes)

    compressed_path = linux_apps_dir / "index.db.zst"
    compressed_path.write_bytes(compressed_db)

    # Remove uncompressed database
    db_path.unlink()

    db_hash = hashlib.sha256(compressed_db).hexdigest()
    logger.info(f"Database compressed: {len(db_bytes)} -> {len(compressed_db)} bytes")

    # Write compressed JSON files
    if not args.no_compress:
        try:
            write_compressed_json(unique_apps, output_dir)
        except ImportError:
            logger.warning("zstandard not available, skipping compressed JSON output")

    # Create local manifest
    manifest = {
        "count": len(unique_apps),
        "index_sha256": db_hash,
        "index_size": len(compressed_db),
        "created_at": datetime.utcnow().isoformat() + "Z",
    }
    manifest_path = linux_apps_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))

    logger.info("Linux apps sync complete!")

    # Print summary
    by_source = {}
    for app in unique_apps:
        by_source[app.source] = by_source.get(app.source, 0) + 1

    print(f"\nSummary:")
    print(f"  Total apps: {len(unique_apps)}")
    for source, count in sorted(by_source.items()):
        print(f"  {source}: {count}")
    print(f"  Database: {compressed_path}")

if __name__ == '__main__':
    main()
