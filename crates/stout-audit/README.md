# stout-audit

Vulnerability auditing for stout packages.

## Overview

This crate provides vulnerability scanning capabilities for packages managed by stout. It checks installed packages against known CVE databases and security advisories.

## Features

- Scans installed packages for known vulnerabilities
- Fetches and caches vulnerability database
- Supports both formula and cask auditing
- Integrates with stout-index and stout-state

## Usage

This crate is primarily used internally by the `stout` CLI through the `stout audit` command.

```rust
use stout_audit::Auditor;

let auditor = Auditor::new()?;
let vulnerabilities = auditor.scan_installed().await?;
```

## License

MIT License - see the repository root for details.
