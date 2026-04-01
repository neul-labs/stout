//! Audit report types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Severity level of a vulnerability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn parse_severity(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Self::Low),
            "medium" | "moderate" => Some(Self::Medium),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Low => "blue",
            Self::Medium => "yellow",
            Self::High => "red",
            Self::Critical => "magenta",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single vulnerability finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Vulnerability ID (CVE-xxx or GHSA-xxx)
    pub id: String,

    /// Formula name
    pub formula: String,

    /// Installed version
    pub installed_version: String,

    /// Summary of the vulnerability
    pub summary: Option<String>,

    /// Severity level
    pub severity: Option<Severity>,

    /// Fixed version (if known)
    pub fixed_version: Option<String>,

    /// Affected version range
    pub affected_versions: Option<String>,

    /// Reference URLs
    pub references: Vec<String>,
}

impl Finding {
    /// Get a display-friendly severity string
    pub fn severity_display(&self) -> &str {
        self.severity.map(|s| s.as_str()).unwrap_or("unknown")
    }
}

/// Audit report containing all findings
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Findings grouped by formula
    pub findings: HashMap<String, Vec<Finding>>,

    /// Count of each severity level
    pub severity_counts: SeverityCounts,

    /// Formulas that were scanned
    pub scanned_formulas: Vec<String>,

    /// Formulas without vulnerability data
    pub unmapped_formulas: Vec<String>,
}

/// Counts of vulnerabilities by severity
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SeverityCounts {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub unknown: usize,
}

impl SeverityCounts {
    pub fn total(&self) -> usize {
        self.critical + self.high + self.medium + self.low + self.unknown
    }

    pub fn increment(&mut self, severity: Option<Severity>) {
        match severity {
            Some(Severity::Critical) => self.critical += 1,
            Some(Severity::High) => self.high += 1,
            Some(Severity::Medium) => self.medium += 1,
            Some(Severity::Low) => self.low += 1,
            None => self.unknown += 1,
        }
    }
}

impl AuditReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a finding to the report
    pub fn add_finding(&mut self, finding: Finding) {
        self.severity_counts.increment(finding.severity);
        self.findings
            .entry(finding.formula.clone())
            .or_default()
            .push(finding);
    }

    /// Check if there are any findings
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }

    /// Total number of findings
    pub fn total_findings(&self) -> usize {
        self.severity_counts.total()
    }

    /// Check if any findings exceed the given severity threshold
    pub fn exceeds_threshold(&self, threshold: Severity) -> bool {
        match threshold {
            Severity::Low => self.severity_counts.total() > 0,
            Severity::Medium => {
                self.severity_counts.medium > 0
                    || self.severity_counts.high > 0
                    || self.severity_counts.critical > 0
            }
            Severity::High => self.severity_counts.high > 0 || self.severity_counts.critical > 0,
            Severity::Critical => self.severity_counts.critical > 0,
        }
    }

    /// Get all findings sorted by severity (critical first)
    pub fn sorted_findings(&self) -> Vec<&Finding> {
        let mut findings: Vec<_> = self.findings.values().flatten().collect();
        findings.sort_by(|a, b| {
            let sev_a = a.severity.unwrap_or(Severity::Low);
            let sev_b = b.severity.unwrap_or(Severity::Low);
            sev_b.cmp(&sev_a) // Descending order (critical first)
        });
        findings
    }
}
