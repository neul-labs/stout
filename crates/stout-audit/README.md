# stout-audit

[![Crates.io](https://img.shields.io/crates/v/stout-audit)](https://crates.io/crates/stout-audit)
[![Docs.rs](https://docs.rs/stout-audit/badge.svg)](https://docs.rs/stout-audit)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](../../LICENSE)

Vulnerability scanning and security auditing for installed packages, with CVE matching and advisory database integration.

**Keywords:** security, vulnerability-scanning, cve, audit, package-security, rust, homebrew, vulnerability-database

## Why stout-audit?

Security vulnerabilities in dependencies are one of the most common attack vectors. `stout-audit` scans your installed packages against known CVE databases and security advisories, telling you exactly what's vulnerable and how to fix it. It runs locally with no external API calls during scanning (the vulnerability database is cached locally).

This crate powers the `stout audit` command but is designed as a reusable security library for any Rust project that needs to check packages against vulnerability databases.

## Features

- **CVE Matching** — Scan installed packages against the National Vulnerability Database
- **Advisory Integration** — Check against GitHub Security Advisories and OSV
- **Local Database** — Download and cache vulnerability data for offline scanning
- **Formula & Cask Auditing** — Scan both CLI tools and applications
- **Severity Scoring** — CVSS v3 scoring with critical/high/medium/low classification
- **Remediation Hints** — Suggest upgrades that fix vulnerabilities
- **Fast Scanning** — SQLite-backed local database queries in milliseconds
- **CI/CD Ready** — Exit codes and JSON output for automation

## Installation

```bash
cargo add stout-audit
```

Or in your `Cargo.toml`:

```toml
[dependencies]
stout-audit = "0.2"
```

## Quick Start

```rust
use stout_audit::Auditor;

// Create an auditor (downloads vulnerability DB if needed)
let auditor = Auditor::new().await?;

// Scan all installed packages
let report = auditor.scan_installed().await?;

for vuln in report.vulnerabilities {
    println!("{}: CVE-{} (Severity: {:?})", 
        vuln.package_name, 
        vuln.cve_id,
        vuln.severity
    );
    println!("  Fixed in: {}", vuln.fixed_in.as_deref().unwrap_or("unknown"));
}
```

## API Overview

### Creating an Auditor

```rust
use stout_audit::{Auditor, AuditConfig};

// Default auditor with auto-updating vulnerability DB
let auditor = Auditor::new().await?;

// With custom cache directory
let config = AuditConfig {
    vuln_db_path: "~/.stout/vulns".into(),
    auto_update: true,
    update_interval: 86400, // Check for updates daily
};
let auditor = Auditor::with_config(config).await?;

// Offline mode (use existing cached DB)
let auditor = Auditor::offline().await?;
```

### Scanning Packages

```rust
// Scan all installed packages
let report = auditor.scan_installed().await?;

// Scan specific packages
let report = auditor.scan_packages(&["openssl", "curl", "node"]).await?;

// Scan a single package
let report = auditor.scan_package("openssl").await?;
```

### Working with Reports

```rust
let report = auditor.scan_installed().await?;

// Summary
println!("Total packages scanned: {}", report.scanned_count);
println!("Vulnerabilities found: {}", report.vulnerability_count);
println!("Critical: {}", report.by_severity(Severity::Critical));
println!("High: {}", report.by_severity(Severity::High));

// Iterate vulnerabilities
for vuln in &report.vulnerabilities {
    println!("Package: {}", vuln.package_name);
    println!("Installed version: {}", vuln.installed_version);
    println!("CVE: {}", vuln.cve_id);
    println!("Severity: {:?} (Score: {})", vuln.severity, vuln.cvss_score);
    println!("Description: {}", vuln.description);
    
    if let Some(fixed) = &vuln.fixed_in {
        println!("Fixed in version: {}", fixed);
        println!("Action: Upgrade to {}", fixed);
    }
    
    if let Some(url) = &vuln.advisory_url {
        println!("More info: {}", url);
    }
}
```

### Filtering and Output

```rust
// Filter by severity
let critical = report.filter_by_severity(Severity::Critical);
let high_and_up = report.filter_by_severity_at_least(Severity::High);

// Filter by package
let openssl_vulns = report.for_package("openssl");

// Export as JSON
let json = report.to_json()?;
println!("{}", json);

// Export as SARIF (for GitHub/CodeQL integration)
let sarif = report.to_sarif()?;
```

### Updating the Vulnerability Database

```rust
// Manually update the vulnerability database
let updated = auditor.update_database().await?;
if updated {
    println!("Vulnerability database updated");
} else {
    println!("Database is already up to date");
}

// Get database info
let meta = auditor.database_metadata()?;
println!("Last updated: {}", meta.last_updated);
println!("CVE count: {}", meta.cve_count);
println!("Version: {}", meta.version);
```

### Checking Before Installation

```rust
// Check if a package version has known vulnerabilities
let has_vulns = auditor.has_vulnerabilities("openssl", "1.1.1")?;
if has_vulns {
    println!("Warning: openssl 1.1.1 has known vulnerabilities");
}

// Get vulnerabilities for a specific version
let vulns = auditor.vulnerabilities_for("openssl", "1.1.1")?;
```

### CI/CD Integration

```rust
use stout_audit::{Auditor, CiConfig};

// Configure for CI: fail on high/critical, output JSON
let config = CiConfig {
    fail_on_severity: Some(Severity::High),
    output_format: OutputFormat::Json,
};

let auditor = Auditor::new().await?;
let report = auditor.scan_installed().await?;

if report.has_severity_at_least(Severity::High) {
    eprintln!("High or critical vulnerabilities found!");
    std::process::exit(1);
}
```

## Vulnerability Database

The local vulnerability database is a SQLite file containing:

- CVE entries with CVSS scores
- Affected package names and version ranges
- Fix versions where known
- Advisory URLs

Database sources:
- NVD (National Vulnerability Database)
- GitHub Security Advisories
- OSV (Open Source Vulnerabilities)

The database is updated incrementally and typically grows by ~5-10MB per month.

## Integration with the Stout Ecosystem

`stout-audit` is the security layer of stout:

- **stout-index** provides package names and versions to scan
- **stout-state** tells the auditor what's installed
- **stout-install** can be configured to warn or block on vulnerable packages
- **stout-bundle** integrates audit checks into bundle workflows

You can use `stout-audit` standalone for any project that needs local vulnerability scanning.

## License

MIT License — see the [repository root](../../LICENSE) for details.
