//! Audit command - scan for vulnerabilities in installed packages

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_audit::{AuditReport, Severity, VulnDatabase, VulnDatabaseConfig};
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Packages to audit (defaults to all installed)
    #[arg()]
    pub packages: Vec<String>,

    /// Update the vulnerability database before scanning
    #[arg(long)]
    pub update: bool,

    /// Output format (text, json)
    #[arg(long, short, default_value = "text")]
    pub format: OutputFormat,

    /// Minimum severity to report (low, medium, high, critical)
    #[arg(long, default_value = "low")]
    pub severity: SeverityArg,

    /// Fail if vulnerabilities are found at or above this severity
    #[arg(long)]
    pub fail_on: Option<SeverityArg>,

    /// Show packages without vulnerability data
    #[arg(long)]
    pub show_unmapped: bool,
}

#[derive(Clone, Debug, Default)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

impl std::str::FromStr for OutputFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            _ => Err(format!("Unknown format: {}", s)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SeverityArg(Severity);

impl Default for SeverityArg {
    fn default() -> Self {
        Self(Severity::Low)
    }
}

impl std::str::FromStr for SeverityArg {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(Self(Severity::Low)),
            "medium" | "moderate" => Ok(Self(Severity::Medium)),
            "high" => Ok(Self(Severity::High)),
            "critical" => Ok(Self(Severity::Critical)),
            _ => Err(format!(
                "Unknown severity: {} (use: low, medium, high, critical)",
                s
            )),
        }
    }
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    // Load installed packages
    let installed = InstalledPackages::load(&paths)?;

    // Determine which packages to audit
    let packages_to_audit: Vec<(String, String)> = if args.packages.is_empty() {
        // Audit all installed packages
        installed
            .iter()
            .map(|(name, info)| (name.clone(), info.version.clone()))
            .collect()
    } else {
        // Audit specified packages
        args.packages
            .iter()
            .filter_map(|name| {
                installed
                    .get(name)
                    .map(|info| (name.clone(), info.version.clone()))
            })
            .collect()
    };

    if packages_to_audit.is_empty() {
        println!("{}", style("No packages to audit").yellow());
        return Ok(());
    }

    // Get or download vulnerability database
    let config = VulnDatabaseConfig::default();

    let db = if args.update || !VulnDatabase::exists(&config) {
        println!("{}", style("Updating vulnerability database...").dim());
        VulnDatabase::download_and_open(config).await?
    } else {
        match VulnDatabase::open(config.clone()) {
            Ok(db) => db,
            Err(_) => {
                println!("{}", style("Downloading vulnerability database...").dim());
                VulnDatabase::download_and_open(config).await?
            }
        }
    };

    // Run the audit
    println!(
        "\n{} {} packages for vulnerabilities...\n",
        style("Auditing").cyan().bold(),
        packages_to_audit.len()
    );

    let report = db.audit_packages(&packages_to_audit)?;

    // Output results
    match args.format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        OutputFormat::Text => {
            print_text_report(&report, args.severity.0, args.show_unmapped);
        }
    }

    // Check fail-on threshold
    if let Some(threshold) = args.fail_on {
        if report.exceeds_threshold(threshold.0) {
            bail!(
                "Found vulnerabilities at or above {} severity",
                threshold.0.as_str()
            );
        }
    }

    Ok(())
}

fn print_text_report(report: &AuditReport, min_severity: Severity, show_unmapped: bool) {
    if !report.has_findings() {
        println!(
            "{}",
            style("No known vulnerabilities found!").green().bold()
        );
        println!("  Scanned {} packages", report.scanned_formulas.len());

        if !report.unmapped_formulas.is_empty() {
            println!(
                "  {} packages have no vulnerability data",
                report.unmapped_formulas.len()
            );
        }
        println!();
        return;
    }

    // Print findings
    let findings = report.sorted_findings();
    for finding in findings {
        let severity = finding.severity.unwrap_or(Severity::Low);
        if severity < min_severity {
            continue;
        }

        // Severity badge
        let sev_display = match severity {
            Severity::Critical => style("CRITICAL").magenta().bold(),
            Severity::High => style("HIGH").red().bold(),
            Severity::Medium => style("MEDIUM").yellow().bold(),
            Severity::Low => style("LOW").blue(),
        };

        println!(
            "{} {} in {} {}",
            sev_display,
            style(&finding.id).cyan(),
            style(&finding.formula).white().bold(),
            style(format!("({})", finding.installed_version)).dim()
        );

        if let Some(summary) = &finding.summary {
            // Truncate long summaries
            let summary = if summary.len() > 100 {
                format!("{}...", &summary[..100])
            } else {
                summary.clone()
            };
            println!("  {}", summary);
        }

        if let Some(fixed) = &finding.fixed_version {
            println!("  {} {}", style("Fix:").green(), fixed);
        }

        if !finding.references.is_empty() {
            println!(
                "  {} {}",
                style("More info:").dim(),
                finding.references.first().unwrap()
            );
        }

        println!();
    }

    // Summary
    let counts = &report.severity_counts;
    println!("{}", style("Summary").bold().underlined());

    if counts.critical > 0 {
        println!("  {} critical", style(counts.critical).magenta().bold());
    }
    if counts.high > 0 {
        println!("  {} high", style(counts.high).red().bold());
    }
    if counts.medium > 0 {
        println!("  {} medium", style(counts.medium).yellow());
    }
    if counts.low > 0 {
        println!("  {} low", style(counts.low).blue());
    }
    if counts.unknown > 0 {
        println!("  {} unknown severity", style(counts.unknown).dim());
    }

    println!();
    println!(
        "  {} total vulnerabilities in {} packages",
        style(report.total_findings()).white().bold(),
        report.findings.len()
    );

    // Show unmapped packages if requested
    if show_unmapped && !report.unmapped_formulas.is_empty() {
        println!();
        println!("{}", style("Packages without vulnerability data:").dim());
        for formula in &report.unmapped_formulas {
            println!("  - {}", formula);
        }
    }

    println!();
}
