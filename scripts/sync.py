#!/usr/bin/env python3
"""
brewx index sync script.

Fetches formula data from Homebrew API and builds:
1. SQLite index database (index.db)
2. Individual formula JSON files (formulas/<name>.json.zst)
3. Manifest file (manifest.json)
"""

import argparse
import hashlib
import json
import logging
import sqlite3
import sys
from datetime import datetime
from pathlib import Path
from typing import Any

import requests
import zstandard as zstd

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s [%(levelname)s] %(message)s",
    datefmt="%H:%M:%S",
)
log = logging.getLogger(__name__)

# Homebrew API endpoints
HOMEBREW_FORMULA_API = "https://formulae.brew.sh/api/formula.json"
HOMEBREW_CASK_API = "https://formulae.brew.sh/api/cask.json"

# Compression level (19 = max)
ZSTD_LEVEL = 19


def fetch_homebrew_formulas() -> list[dict]:
    """Fetch all formulas from Homebrew API."""
    log.info(f"Fetching formulas from {HOMEBREW_FORMULA_API}")
    response = requests.get(HOMEBREW_FORMULA_API, timeout=60)
    response.raise_for_status()
    formulas = response.json()
    log.info(f"Fetched {len(formulas)} formulas")
    return formulas


def transform_formula(hb_formula: dict) -> dict:
    """Transform Homebrew formula JSON to brewx format."""
    # Extract bottles
    bottles = {}
    if bottle_data := hb_formula.get("bottle", {}).get("stable", {}).get("files", {}):
        for platform, data in bottle_data.items():
            bottles[platform] = {
                "url": data.get("url", ""),
                "sha256": data.get("sha256", ""),
                "cellar": data.get("cellar", "/opt/homebrew/Cellar"),
            }

    # Build brewx format
    return {
        "name": hb_formula["name"],
        "version": hb_formula.get("versions", {}).get("stable", ""),
        "revision": hb_formula.get("revision", 0),
        "desc": hb_formula.get("desc"),
        "homepage": hb_formula.get("homepage"),
        "license": hb_formula.get("license"),
        "tap": hb_formula.get("tap", "homebrew/core"),
        "urls": {
            "stable": {
                "url": hb_formula.get("urls", {}).get("stable", {}).get("url", ""),
                "sha256": hb_formula.get("urls", {}).get("stable", {}).get("checksum", ""),
            }
            if hb_formula.get("urls", {}).get("stable")
            else None,
            "head": hb_formula.get("urls", {}).get("head", {}).get("url"),
        },
        "bottles": bottles,
        "dependencies": {
            "runtime": hb_formula.get("dependencies", []),
            "build": hb_formula.get("build_dependencies", []),
            "test": hb_formula.get("test_dependencies", []),
            "optional": hb_formula.get("optional_dependencies", []),
            "recommended": hb_formula.get("recommended_dependencies", []),
        },
        "aliases": hb_formula.get("aliases", []),
        "conflicts_with": hb_formula.get("conflicts_with", []),
        "caveats": hb_formula.get("caveats"),
        "flags": {
            "keg_only": hb_formula.get("keg_only", False),
            "deprecated": hb_formula.get("deprecated", False),
            "disabled": hb_formula.get("disabled", False),
            "has_post_install": hb_formula.get("post_install_defined", False),
        },
        "service": hb_formula.get("service"),
        "meta": {
            "ruby_source_path": hb_formula.get("ruby_source_path"),
            "tap_git_head": hb_formula.get("tap_git_head"),
        },
    }


def create_database(db_path: Path) -> sqlite3.Connection:
    """Create SQLite database with schema."""
    log.info(f"Creating database at {db_path}")

    # Remove existing database
    if db_path.exists():
        db_path.unlink()

    conn = sqlite3.connect(db_path)

    # Create schema
    conn.executescript(
        """
        -- Formula metadata (fast queries, search, listing)
        CREATE TABLE formulas (
            name TEXT PRIMARY KEY,
            version TEXT NOT NULL,
            revision INTEGER DEFAULT 0,
            desc TEXT,
            homepage TEXT,
            license TEXT,
            tap TEXT DEFAULT 'homebrew/core',
            deprecated INTEGER DEFAULT 0,
            disabled INTEGER DEFAULT 0,
            has_bottle INTEGER DEFAULT 1,
            json_hash TEXT,
            updated_at INTEGER
        );

        -- Full-text search
        CREATE VIRTUAL TABLE formulas_fts USING fts5(
            name, desc,
            content='formulas',
            content_rowid='rowid'
        );

        -- Triggers to keep FTS in sync
        CREATE TRIGGER formulas_ai AFTER INSERT ON formulas BEGIN
            INSERT INTO formulas_fts(rowid, name, desc) VALUES (NEW.rowid, NEW.name, NEW.desc);
        END;

        CREATE TRIGGER formulas_ad AFTER DELETE ON formulas BEGIN
            INSERT INTO formulas_fts(formulas_fts, rowid, name, desc) VALUES('delete', OLD.rowid, OLD.name, OLD.desc);
        END;

        CREATE TRIGGER formulas_au AFTER UPDATE ON formulas BEGIN
            INSERT INTO formulas_fts(formulas_fts, rowid, name, desc) VALUES('delete', OLD.rowid, OLD.name, OLD.desc);
            INSERT INTO formulas_fts(rowid, name, desc) VALUES (NEW.rowid, NEW.name, NEW.desc);
        END;

        -- Dependencies
        CREATE TABLE dependencies (
            formula TEXT NOT NULL,
            dep_name TEXT NOT NULL,
            dep_type TEXT NOT NULL,
            PRIMARY KEY (formula, dep_name, dep_type),
            FOREIGN KEY (formula) REFERENCES formulas(name)
        );

        -- Bottle availability matrix
        CREATE TABLE bottles (
            formula TEXT NOT NULL,
            platform TEXT NOT NULL,
            PRIMARY KEY (formula, platform),
            FOREIGN KEY (formula) REFERENCES formulas(name)
        );

        -- Aliases
        CREATE TABLE aliases (
            alias TEXT PRIMARY KEY,
            formula TEXT NOT NULL,
            FOREIGN KEY (formula) REFERENCES formulas(name)
        );

        -- Index metadata
        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        -- Indexes
        CREATE INDEX idx_dependencies_formula ON dependencies(formula);
        CREATE INDEX idx_dependencies_dep ON dependencies(dep_name);
        CREATE INDEX idx_bottles_formula ON bottles(formula);
        CREATE INDEX idx_formulas_tap ON formulas(tap);
    """
    )

    return conn


def compress_json(data: dict) -> bytes:
    """Compress JSON data with zstd."""
    json_bytes = json.dumps(data, separators=(",", ":")).encode("utf-8")
    compressor = zstd.ZstdCompressor(level=ZSTD_LEVEL)
    return compressor.compress(json_bytes)


def sha256_bytes(data: bytes) -> str:
    """Calculate SHA256 hash of bytes."""
    return hashlib.sha256(data).hexdigest()


def sync(output_dir: Path, full: bool = False) -> dict:
    """
    Sync formulas from Homebrew API.

    Returns manifest dict.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    formulas_dir = output_dir / "formulas"
    formulas_dir.mkdir(exist_ok=True)

    # Fetch formulas
    hb_formulas = fetch_homebrew_formulas()

    # Create database
    db_path = output_dir / "index.db"
    conn = create_database(db_path)
    cursor = conn.cursor()

    timestamp = int(datetime.utcnow().timestamp())
    formula_count = 0

    log.info("Processing formulas...")

    for hb_formula in hb_formulas:
        name = hb_formula["name"]

        # Transform to brewx format
        brewx_formula = transform_formula(hb_formula)

        # Compress and write individual JSON
        compressed = compress_json(brewx_formula)
        json_hash = sha256_bytes(compressed)

        # Write to formulas/<first_letter>/<name>.json.zst
        first_letter = name[0].lower()
        letter_dir = formulas_dir / first_letter
        letter_dir.mkdir(exist_ok=True)

        formula_path = letter_dir / f"{name}.json.zst"
        formula_path.write_bytes(compressed)

        # Insert into index
        version = brewx_formula["version"]
        revision = brewx_formula["revision"]
        desc = brewx_formula.get("desc")
        homepage = brewx_formula.get("homepage")
        license_str = brewx_formula.get("license")
        tap = brewx_formula.get("tap", "homebrew/core")
        deprecated = 1 if brewx_formula["flags"]["deprecated"] else 0
        disabled = 1 if brewx_formula["flags"]["disabled"] else 0
        has_bottle = 1 if brewx_formula["bottles"] else 0

        cursor.execute(
            """
            INSERT INTO formulas (name, version, revision, desc, homepage, license, tap,
                                  deprecated, disabled, has_bottle, json_hash, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                name,
                version,
                revision,
                desc,
                homepage,
                license_str,
                tap,
                deprecated,
                disabled,
                has_bottle,
                json_hash,
                timestamp,
            ),
        )

        # Insert dependencies
        deps = brewx_formula["dependencies"]
        for dep_type, dep_list in deps.items():
            for dep_name in dep_list:
                cursor.execute(
                    "INSERT OR IGNORE INTO dependencies (formula, dep_name, dep_type) VALUES (?, ?, ?)",
                    (name, dep_name, dep_type),
                )

        # Insert bottle platforms
        for platform in brewx_formula["bottles"]:
            cursor.execute(
                "INSERT OR IGNORE INTO bottles (formula, platform) VALUES (?, ?)",
                (name, platform),
            )

        # Insert aliases
        for alias in brewx_formula.get("aliases", []):
            cursor.execute(
                "INSERT OR IGNORE INTO aliases (alias, formula) VALUES (?, ?)",
                (alias, name),
            )

        formula_count += 1

        if formula_count % 500 == 0:
            log.info(f"  Processed {formula_count} formulas...")

    # Set metadata
    version = datetime.utcnow().strftime("%Y.%m.%d.%H%M")
    cursor.execute("INSERT INTO meta (key, value) VALUES ('version', ?)", (version,))
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('created_at', ?)",
        (datetime.utcnow().isoformat(),),
    )
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('formula_count', ?)", (str(formula_count),)
    )

    conn.commit()
    conn.close()

    log.info(f"Processed {formula_count} formulas")

    # Compress database
    log.info("Compressing database...")
    db_bytes = db_path.read_bytes()
    compressor = zstd.ZstdCompressor(level=ZSTD_LEVEL)
    compressed_db = compressor.compress(db_bytes)

    compressed_path = output_dir / "index.db.zst"
    compressed_path.write_bytes(compressed_db)

    db_hash = sha256_bytes(compressed_db)
    log.info(
        f"Database: {len(db_bytes)} bytes -> {len(compressed_db)} bytes ({len(compressed_db)/len(db_bytes)*100:.1f}%)"
    )

    # Create manifest
    manifest = {
        "version": version,
        "index_version": version,
        "index_sha256": db_hash,
        "index_size": len(compressed_db),
        "formula_count": formula_count,
        "created_at": datetime.utcnow().isoformat() + "Z",
        "homebrew_commit": hb_formulas[0].get("tap_git_head") if hb_formulas else None,
    }

    manifest_path = output_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))

    log.info(f"Created manifest: {manifest_path}")
    log.info(f"Version: {version}")

    return manifest


def main():
    parser = argparse.ArgumentParser(description="Sync brewx index from Homebrew API")
    parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("dist"),
        help="Output directory (default: dist)",
    )
    parser.add_argument(
        "--full",
        action="store_true",
        help="Full rebuild (ignore incremental)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be done without writing files",
    )

    args = parser.parse_args()

    if args.dry_run:
        log.info("Dry run mode - no files will be written")
        # Just fetch and show stats
        formulas = fetch_homebrew_formulas()
        log.info(f"Would process {len(formulas)} formulas")
        return

    manifest = sync(args.output, full=args.full)

    log.info("Sync complete!")
    log.info(f"  Version: {manifest['version']}")
    log.info(f"  Formulas: {manifest['formula_count']}")
    log.info(f"  Index size: {manifest['index_size']} bytes")


if __name__ == "__main__":
    main()
