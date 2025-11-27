//! Vulnerability database management

use crate::error::{Error, Result};
use crate::report::{AuditReport, Finding, Severity};
use crate::version::version_affected;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Configuration for the vulnerability database
#[derive(Debug, Clone)]
pub struct VulnDatabaseConfig {
    /// Base URL for downloading the vulnerability index
    pub base_url: String,

    /// Path to the local cache directory
    pub cache_dir: PathBuf,

    /// Whether to auto-update the index
    pub auto_update: bool,
}

impl Default for VulnDatabaseConfig {
    fn default() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("brewx")
            .join("vuln");

        Self {
            base_url: "https://raw.githubusercontent.com/neul-labs/brewx-index/main/vulnerabilities".to_string(),
            cache_dir,
            auto_update: true,
        }
    }
}

/// Vulnerability database for querying package vulnerabilities
pub struct VulnDatabase {
    conn: Connection,
    config: VulnDatabaseConfig,
}

/// Vulnerability record from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub summary: Option<String>,
    pub details: Option<String>,
    pub severity: Option<Severity>,
    pub published: Option<String>,
    pub modified: Option<String>,
    pub references: Vec<String>,
}

/// Affected package mapping
#[derive(Debug, Clone)]
pub struct AffectedPackage {
    pub vuln_id: String,
    pub formula: String,
    pub ecosystem: Option<String>,
    pub package: Option<String>,
    pub affected_versions: Option<String>,
    pub fixed_version: Option<String>,
}

impl VulnDatabase {
    /// Open the vulnerability database from the default cache location
    pub fn open(config: VulnDatabaseConfig) -> Result<Self> {
        let db_path = config.cache_dir.join("vulnerabilities.db");

        if !db_path.exists() {
            return Err(Error::DatabaseNotFound(db_path));
        }

        Self::open_path(&db_path, config)
    }

    /// Open the vulnerability database from a specific path
    pub fn open_path(path: &Path, config: VulnDatabaseConfig) -> Result<Self> {
        debug!("Opening vulnerability database at {:?}", path);

        let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

        // Enable query optimization
        conn.execute_batch(
            "PRAGMA query_only = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA mmap_size = 268435456;",
        )?;

        Ok(Self { conn, config })
    }

    /// Open from a compressed database file
    pub async fn open_compressed(
        compressed_path: &Path,
        config: VulnDatabaseConfig,
    ) -> Result<Self> {
        let db_path = config.cache_dir.join("vulnerabilities.db");

        // Ensure cache directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Decompress the database
        let compressed = std::fs::read(compressed_path)?;
        let decompressed = zstd::decode_all(compressed.as_slice())
            .map_err(|e| Error::Decompress(e.to_string()))?;

        std::fs::write(&db_path, decompressed)?;

        Self::open_path(&db_path, config)
    }

    /// Download and open the latest vulnerability database
    pub async fn download_and_open(config: VulnDatabaseConfig) -> Result<Self> {
        let db_url = format!("{}/vulnerabilities.db.zst", config.base_url);
        let db_path = config.cache_dir.join("vulnerabilities.db");

        info!("Downloading vulnerability database from {}", db_url);

        // Ensure cache directory exists
        std::fs::create_dir_all(&config.cache_dir)?;

        // Download compressed database
        let client = reqwest::Client::new();
        let response = client.get(&db_url).send().await?;
        let compressed = response.bytes().await?;

        // Decompress
        let decompressed = zstd::decode_all(compressed.as_ref())
            .map_err(|e| Error::Decompress(e.to_string()))?;

        std::fs::write(&db_path, decompressed)?;

        Self::open_path(&db_path, config)
    }

    /// Check if the database exists
    pub fn exists(config: &VulnDatabaseConfig) -> bool {
        config.cache_dir.join("vulnerabilities.db").exists()
    }

    /// Get database version
    pub fn version(&self) -> Result<String> {
        let version: String = self.conn.query_row(
            "SELECT value FROM meta WHERE key = 'version'",
            [],
            |row| row.get(0),
        )?;
        Ok(version)
    }

    /// Get database statistics
    pub fn stats(&self) -> Result<DatabaseStats> {
        let vuln_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM vulnerabilities", [], |row| {
                row.get(0)
            })?;

        let affected_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM affected_packages", [], |row| {
                row.get(0)
            })?;

        let formula_count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT formula) FROM affected_packages",
            [],
            |row| row.get(0),
        )?;

        Ok(DatabaseStats {
            vulnerability_count: vuln_count as usize,
            affected_mapping_count: affected_count as usize,
            formula_count: formula_count as usize,
        })
    }

    /// Get all formulas that have vulnerability data
    pub fn covered_formulas(&self) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT formula FROM affected_packages ORDER BY formula")?;

        let formulas = stmt
            .query_map([], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;

        Ok(formulas)
    }

    /// Check if a formula has vulnerability data
    pub fn has_formula(&self, formula: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM affected_packages WHERE formula = ?",
            [formula],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Get vulnerabilities for a specific formula
    pub fn get_vulnerabilities(&self, formula: &str) -> Result<Vec<(Vulnerability, AffectedPackage)>> {
        let mut stmt = self.conn.prepare(
            "SELECT v.id, v.summary, v.details, v.severity, v.published, v.modified, v.references_json,
                    a.formula, a.ecosystem, a.package, a.affected_versions, a.fixed_version
             FROM vulnerabilities v
             JOIN affected_packages a ON v.id = a.vuln_id
             WHERE a.formula = ?",
        )?;

        let results = stmt
            .query_map([formula], |row| {
                let refs_json: Option<String> = row.get(6)?;
                let references: Vec<String> = refs_json
                    .and_then(|s| serde_json::from_str(&s).ok())
                    .unwrap_or_default();

                let severity_str: Option<String> = row.get(3)?;
                let severity = severity_str.as_deref().and_then(Severity::from_str);

                let vuln = Vulnerability {
                    id: row.get(0)?,
                    summary: row.get(1)?,
                    details: row.get(2)?,
                    severity,
                    published: row.get(4)?,
                    modified: row.get(5)?,
                    references,
                };

                let affected = AffectedPackage {
                    vuln_id: row.get(0)?,
                    formula: row.get(7)?,
                    ecosystem: row.get(8)?,
                    package: row.get(9)?,
                    affected_versions: row.get(10)?,
                    fixed_version: row.get(11)?,
                };

                Ok((vuln, affected))
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(results)
    }

    /// Audit a single formula with its installed version
    pub fn audit_formula(
        &self,
        formula: &str,
        installed_version: &str,
    ) -> Result<Vec<Finding>> {
        let vulns = self.get_vulnerabilities(formula)?;
        let mut findings = Vec::new();

        for (vuln, affected) in vulns {
            // Check if installed version is affected
            let is_affected = version_affected(
                installed_version,
                affected.affected_versions.as_deref(),
                affected.fixed_version.as_deref(),
            );

            if is_affected {
                findings.push(Finding {
                    id: vuln.id,
                    formula: formula.to_string(),
                    installed_version: installed_version.to_string(),
                    summary: vuln.summary,
                    severity: vuln.severity,
                    fixed_version: affected.fixed_version,
                    affected_versions: affected.affected_versions,
                    references: vuln.references,
                });
            }
        }

        Ok(findings)
    }

    /// Audit multiple formulas with their versions
    pub fn audit_packages(
        &self,
        packages: &[(String, String)], // (formula, version)
    ) -> Result<AuditReport> {
        let mut report = AuditReport::new();

        for (formula, version) in packages {
            report.scanned_formulas.push(formula.clone());

            if !self.has_formula(formula)? {
                report.unmapped_formulas.push(formula.clone());
                continue;
            }

            let findings = self.audit_formula(formula, version)?;
            for finding in findings {
                report.add_finding(finding);
            }
        }

        Ok(report)
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub vulnerability_count: usize,
    pub affected_mapping_count: usize,
    pub formula_count: usize,
}
