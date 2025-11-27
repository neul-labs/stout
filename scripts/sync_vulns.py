#!/usr/bin/env python3
"""
brewx vulnerability index sync script.

Fetches vulnerability data from OSV (Open Source Vulnerabilities) and builds:
1. SQLite vulnerability database (vulnerabilities.db)
2. Manifest file with checksums

Data sources:
- OSV API: https://api.osv.dev/v1/query
- Formula-to-ecosystem mappings from local mappings file
"""

import argparse
import hashlib
import json
import logging
import sqlite3
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
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

# OSV API endpoint
OSV_API = "https://api.osv.dev/v1/query"

# Compression level (19 = max)
ZSTD_LEVEL = 19

# Default ecosystem mappings for common formulas
# Maps formula name -> (ecosystem, package_name)
DEFAULT_MAPPINGS = {
    # Languages / Runtimes
    "node": ("npm", "node"),
    "node@20": ("npm", "node"),
    "node@22": ("npm", "node"),
    "python@3.11": ("PyPI", "cpython"),
    "python@3.12": ("PyPI", "cpython"),
    "python@3.13": ("PyPI", "cpython"),
    "ruby": ("RubyGems", "ruby"),
    "go": ("Go", "stdlib"),
    "rust": ("crates.io", "rust"),
    # Common tools
    "openssl": ("OSS-Fuzz", "openssl"),
    "openssl@3": ("OSS-Fuzz", "openssl"),
    "curl": ("OSS-Fuzz", "curl"),
    "wget": ("OSS-Fuzz", "wget"),
    "git": ("OSS-Fuzz", "git"),
    "sqlite": ("OSS-Fuzz", "sqlite3"),
    "zlib": ("OSS-Fuzz", "zlib"),
    "libpng": ("OSS-Fuzz", "libpng"),
    "libjpeg": ("OSS-Fuzz", "libjpeg-turbo"),
    "libxml2": ("OSS-Fuzz", "libxml2"),
    "libxslt": ("OSS-Fuzz", "libxslt"),
    "pcre2": ("OSS-Fuzz", "pcre2"),
    "jq": ("GitHub", "jqlang/jq"),
    "ripgrep": ("crates.io", "ripgrep"),
    "fd": ("crates.io", "fd-find"),
    "bat": ("crates.io", "bat"),
    "exa": ("crates.io", "exa"),
    "eza": ("crates.io", "eza"),
    "fzf": ("Go", "github.com/junegunn/fzf"),
    "gh": ("Go", "github.com/cli/cli"),
    # Databases
    "postgresql": ("OSS-Fuzz", "postgresql"),
    "postgresql@15": ("OSS-Fuzz", "postgresql"),
    "postgresql@16": ("OSS-Fuzz", "postgresql"),
    "mysql": ("OSS-Fuzz", "mysql-server"),
    "redis": ("OSS-Fuzz", "redis"),
    # Web / Network
    "nginx": ("OSS-Fuzz", "nginx"),
    "apache-httpd": ("OSS-Fuzz", "httpd"),
    "haproxy": ("OSS-Fuzz", "haproxy"),
    # Development tools
    "cmake": ("OSS-Fuzz", "cmake"),
    "meson": ("PyPI", "meson"),
    "ninja": ("GitHub", "ninja-build/ninja"),
    # Crypto
    "gnupg": ("OSS-Fuzz", "gnupg"),
    "gpg": ("OSS-Fuzz", "gnupg"),
    "libsodium": ("OSS-Fuzz", "libsodium"),
    # Image / Media
    "ffmpeg": ("OSS-Fuzz", "ffmpeg"),
    "imagemagick": ("OSS-Fuzz", "imagemagick"),
    # JSON / Data
    "yq": ("Go", "github.com/mikefarah/yq"),
    "jsonnet": ("GitHub", "google/jsonnet"),
}


def load_mappings(mappings_file: Path | None) -> dict[str, tuple[str, str]]:
    """Load formula-to-ecosystem mappings."""
    mappings = DEFAULT_MAPPINGS.copy()

    if mappings_file and mappings_file.exists():
        log.info(f"Loading additional mappings from {mappings_file}")
        with open(mappings_file) as f:
            custom = json.load(f)
            for formula, info in custom.items():
                mappings[formula] = (info["ecosystem"], info["package"])

    return mappings


def query_osv(ecosystem: str, package: str) -> list[dict]:
    """Query OSV API for vulnerabilities affecting a package."""
    try:
        response = requests.post(
            OSV_API,
            json={"package": {"name": package, "ecosystem": ecosystem}},
            timeout=30,
        )
        response.raise_for_status()
        data = response.json()
        return data.get("vulns", [])
    except requests.RequestException as e:
        log.debug(f"OSV query failed for {ecosystem}/{package}: {e}")
        return []


def extract_severity(vuln: dict) -> str | None:
    """Extract severity from vulnerability data."""
    # Try CVSS v3 first
    if severities := vuln.get("severity", []):
        for sev in severities:
            if sev.get("type") == "CVSS_V3":
                score_str = sev.get("score", "")
                # Parse CVSS vector string to get base score
                if "CVSS:3" in score_str:
                    # Extract base score from vector
                    parts = score_str.split("/")
                    for part in parts:
                        if part.startswith("CVSS:3"):
                            continue
                        # The format varies, so we estimate from the vector
                    # Fallback to classification from vector components
                    if any(x in score_str for x in ["A:H", "C:H", "I:H"]):
                        return "high"
                    elif any(x in score_str for x in ["A:L", "C:L", "I:L"]):
                        return "low"
                    return "medium"

    # Try database_specific severity
    if db_specific := vuln.get("database_specific", {}):
        if severity := db_specific.get("severity"):
            return severity.lower()

    # Try ecosystem_specific severity
    for affected in vuln.get("affected", []):
        if eco_specific := affected.get("ecosystem_specific", {}):
            if severity := eco_specific.get("severity"):
                return severity.lower()

    return None


def extract_fixed_version(vuln: dict) -> str | None:
    """Extract the fixed version from vulnerability data."""
    for affected in vuln.get("affected", []):
        for rng in affected.get("ranges", []):
            for event in rng.get("events", []):
                if fixed := event.get("fixed"):
                    return fixed
    return None


def extract_affected_versions(vuln: dict) -> str | None:
    """Extract affected version range as a string."""
    ranges = []
    for affected in vuln.get("affected", []):
        # Try versions list first
        if versions := affected.get("versions"):
            return ", ".join(versions[:5])  # Limit to first 5

        # Try ranges
        for rng in affected.get("ranges", []):
            range_str = ""
            for event in rng.get("events", []):
                if introduced := event.get("introduced"):
                    range_str = f">={introduced}"
                if fixed := event.get("fixed"):
                    range_str += f", <{fixed}"
                if last_affected := event.get("last_affected"):
                    range_str += f", <={last_affected}"
            if range_str:
                ranges.append(range_str)

    return "; ".join(ranges) if ranges else None


def create_database(db_path: Path) -> sqlite3.Connection:
    """Create SQLite database with vulnerability schema."""
    log.info(f"Creating database at {db_path}")

    # Remove existing database
    if db_path.exists():
        db_path.unlink()

    conn = sqlite3.connect(db_path)

    # Create schema
    conn.executescript(
        """
        -- Vulnerability records
        CREATE TABLE vulnerabilities (
            id TEXT PRIMARY KEY,              -- CVE-2023-50246 or GHSA-xxx
            summary TEXT,
            details TEXT,
            severity TEXT,                    -- critical, high, medium, low
            published TEXT,
            modified TEXT,
            references_json TEXT              -- JSON array of reference URLs
        );

        -- Affected formula packages
        CREATE TABLE affected_packages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            vuln_id TEXT NOT NULL,
            formula TEXT NOT NULL,            -- brewx formula name
            ecosystem TEXT,                   -- Original ecosystem (npm, PyPI, etc.)
            package TEXT,                     -- Original package name
            affected_versions TEXT,           -- Version range expression
            fixed_version TEXT,
            FOREIGN KEY (vuln_id) REFERENCES vulnerabilities(id)
        );

        -- Formula to vulnerability mapping (quick lookups)
        CREATE INDEX idx_affected_formula ON affected_packages(formula);
        CREATE INDEX idx_affected_vuln ON affected_packages(vuln_id);

        -- Index metadata
        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        -- Statistics
        CREATE TABLE stats (
            ecosystem TEXT PRIMARY KEY,
            vuln_count INTEGER DEFAULT 0,
            package_count INTEGER DEFAULT 0
        );
    """
    )

    return conn


def compress_database(db_path: Path) -> tuple[bytes, str]:
    """Compress database with zstd and return (compressed_bytes, hash)."""
    db_bytes = db_path.read_bytes()
    compressor = zstd.ZstdCompressor(level=ZSTD_LEVEL)
    compressed = compressor.compress(db_bytes)
    hash_val = hashlib.sha256(compressed).hexdigest()
    return compressed, hash_val


def sync_vulnerabilities(
    output_dir: Path,
    mappings: dict[str, tuple[str, str]],
    parallel: int = 4,
) -> dict:
    """
    Sync vulnerabilities from OSV API.

    Returns manifest dict.
    """
    output_dir.mkdir(parents=True, exist_ok=True)

    # Create vulnerabilities directory
    vulns_dir = output_dir / "vulnerabilities"
    vulns_dir.mkdir(exist_ok=True)

    # Create database
    db_path = vulns_dir / "index.db"
    conn = create_database(db_path)
    cursor = conn.cursor()

    timestamp = int(datetime.utcnow().timestamp())
    vuln_count = 0
    affected_count = 0
    package_count = 0
    stats: dict[str, dict] = {}

    log.info(f"Querying OSV for {len(mappings)} formula mappings...")

    # Query OSV in parallel
    results: dict[str, list[dict]] = {}

    def fetch_vulns(formula: str, ecosystem: str, package: str) -> tuple[str, list]:
        vulns = query_osv(ecosystem, package)
        return formula, vulns

    with ThreadPoolExecutor(max_workers=parallel) as executor:
        futures = {
            executor.submit(fetch_vulns, formula, eco, pkg): formula
            for formula, (eco, pkg) in mappings.items()
        }

        for future in as_completed(futures):
            formula = futures[future]
            try:
                _, vulns = future.result()
                if vulns:
                    results[formula] = vulns
                    log.debug(f"  {formula}: {len(vulns)} vulnerabilities")
            except Exception as e:
                log.warning(f"Error fetching {formula}: {e}")

    log.info(f"Found vulnerabilities for {len(results)} formulas")

    # Process results and insert into database
    seen_vulns: set[str] = set()

    for formula, vulns in results.items():
        ecosystem, package = mappings[formula]
        package_count += 1

        # Track ecosystem stats
        if ecosystem not in stats:
            stats[ecosystem] = {"vuln_count": 0, "package_count": 0}
        stats[ecosystem]["package_count"] += 1

        for vuln in vulns:
            vuln_id = vuln.get("id", "")
            if not vuln_id:
                continue

            # Insert vulnerability if not seen
            if vuln_id not in seen_vulns:
                seen_vulns.add(vuln_id)
                vuln_count += 1
                stats[ecosystem]["vuln_count"] += 1

                # Extract references
                references = [
                    ref.get("url") for ref in vuln.get("references", []) if ref.get("url")
                ]

                cursor.execute(
                    """
                    INSERT OR IGNORE INTO vulnerabilities
                    (id, summary, details, severity, published, modified, references_json)
                    VALUES (?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        vuln_id,
                        vuln.get("summary"),
                        vuln.get("details"),
                        extract_severity(vuln),
                        vuln.get("published"),
                        vuln.get("modified"),
                        json.dumps(references) if references else None,
                    ),
                )

            # Insert affected package mapping
            cursor.execute(
                """
                INSERT INTO affected_packages
                (vuln_id, formula, ecosystem, package, affected_versions, fixed_version)
                VALUES (?, ?, ?, ?, ?, ?)
                """,
                (
                    vuln_id,
                    formula,
                    ecosystem,
                    package,
                    extract_affected_versions(vuln),
                    extract_fixed_version(vuln),
                ),
            )
            affected_count += 1

    # Insert stats
    for ecosystem, data in stats.items():
        cursor.execute(
            "INSERT INTO stats (ecosystem, vuln_count, package_count) VALUES (?, ?, ?)",
            (ecosystem, data["vuln_count"], data["package_count"]),
        )

    # Set metadata
    version = datetime.utcnow().strftime("%Y.%m.%d.%H%M")
    cursor.execute("INSERT INTO meta (key, value) VALUES ('version', ?)", (version,))
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('created_at', ?)",
        (datetime.utcnow().isoformat(),),
    )
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('vuln_count', ?)", (str(vuln_count),)
    )
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('affected_count', ?)", (str(affected_count),)
    )
    cursor.execute(
        "INSERT INTO meta (key, value) VALUES ('formula_count', ?)", (str(package_count),)
    )

    conn.commit()
    conn.close()

    log.info(f"Processed {vuln_count} unique vulnerabilities")
    log.info(f"Mapped to {affected_count} formula-vulnerability pairs")
    log.info(f"Covered {package_count} formulas")

    # Compress database
    log.info("Compressing database...")
    compressed, db_hash = compress_database(db_path)

    compressed_path = vulns_dir / "index.db.zst"
    compressed_path.write_bytes(compressed)

    db_size = db_path.stat().st_size

    # Remove uncompressed database
    db_path.unlink()

    log.info(
        f"Database: {db_size} bytes -> {len(compressed)} bytes "
        f"({len(compressed)/db_size*100:.1f}%)"
    )

    # Create manifest
    manifest = {
        "version": version,
        "index_version": version,
        "index_sha256": db_hash,
        "index_size": len(compressed),
        "vulnerability_count": vuln_count,
        "affected_mapping_count": affected_count,
        "formula_count": package_count,
        "ecosystems": list(stats.keys()),
        "created_at": datetime.utcnow().isoformat() + "Z",
    }

    manifest_path = vulns_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2))

    log.info(f"Created manifest: {manifest_path}")

    return manifest


def main():
    parser = argparse.ArgumentParser(
        description="Sync brewx vulnerability index from OSV"
    )
    parser.add_argument(
        "--output",
        "-o",
        type=Path,
        default=Path("dist/vulnerabilities"),
        help="Output directory (default: dist/vulnerabilities)",
    )
    parser.add_argument(
        "--mappings",
        "-m",
        type=Path,
        help="Path to formula_ecosystems.json mappings file",
    )
    parser.add_argument(
        "--parallel",
        "-j",
        type=int,
        default=4,
        help="Number of parallel OSV queries (default: 4)",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Show what would be done without writing files",
    )

    args = parser.parse_args()

    # Load mappings
    mappings = load_mappings(args.mappings)
    log.info(f"Loaded {len(mappings)} formula mappings")

    if args.dry_run:
        log.info("Dry run mode - no files will be written")
        log.info(f"Would query OSV for {len(mappings)} formulas:")
        for formula, (eco, pkg) in sorted(mappings.items())[:20]:
            log.info(f"  {formula} -> {eco}/{pkg}")
        if len(mappings) > 20:
            log.info(f"  ... and {len(mappings) - 20} more")
        return

    manifest = sync_vulnerabilities(args.output, mappings, args.parallel)

    log.info("Sync complete!")
    log.info(f"  Version: {manifest['version']}")
    log.info(f"  Vulnerabilities: {manifest['vulnerability_count']}")
    log.info(f"  Mappings: {manifest['affected_mapping_count']}")
    log.info(f"  Formulas: {manifest['formula_count']}")
    log.info(f"  Index size: {manifest['index_size']} bytes")


if __name__ == "__main__":
    main()
