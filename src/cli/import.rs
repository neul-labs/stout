//! Import command — import existing Homebrew packages into Stout's state

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_install::cellar::{scan_cellar, scan_cellar_package, timestamp_to_iso, CellarPackage};
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Specific packages to import (default: all)
    pub packages: Vec<String>,

    /// Show what would be imported without modifying state
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Re-import packages already tracked by Stout
    #[arg(long)]
    pub overwrite: bool,

    /// Show detailed output for each package
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let mut installed = InstalledPackages::load(&paths)?;

    println!("\n{}...", style("Scanning Cellar").cyan());

    let cellar_packages = if args.packages.is_empty() {
        scan_cellar(&paths.cellar)?
    } else {
        let mut pkgs = Vec::new();
        for name in &args.packages {
            match scan_cellar_package(&paths.cellar, name)? {
                Some(pkg) => pkgs.push(pkg),
                None => {
                    eprintln!("  {} {} not found in Cellar", style("✗").red(), name);
                }
            }
        }
        pkgs
    };

    if cellar_packages.is_empty() {
        println!("\n{}", style("No packages found in Cellar.").dim());
        return Ok(());
    }

    println!(
        "\n{} {} packages:\n",
        style("Importing").cyan(),
        cellar_packages.len()
    );

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = 0u32;

    for pkg in &cellar_packages {
        // Check if already tracked
        if installed.is_installed(&pkg.name) && !args.overwrite {
            if args.verbose {
                println!(
                    "  {} {} {} {}",
                    style("⊘").dim(),
                    pkg.name,
                    style(&pkg.version).dim(),
                    style("(already tracked)").dim()
                );
            }
            skipped += 1;
            continue;
        }

        if args.dry_run {
            println!(
                "  {} {} {}",
                style("✓").green(),
                pkg.name,
                style(&pkg.version).dim()
            );
            imported += 1;
            continue;
        }

        import_cellar_package(&mut installed, pkg);
        imported += 1;

        println!(
            "  {} {} {}",
            style("✓").green(),
            pkg.name,
            style(&pkg.version).dim()
        );
    }

    if !args.dry_run && imported > 0 {
        installed.save(&paths)?;
    }

    println!(
        "\n{} {} packages ({} already tracked, {} errors)",
        if args.dry_run {
            style("Would import").yellow()
        } else {
            style("Imported").green().bold()
        },
        imported,
        skipped,
        errors
    );

    Ok(())
}

/// Import a single CellarPackage into InstalledPackages state.
pub fn import_cellar_package(installed: &mut InstalledPackages, pkg: &CellarPackage) {
    let (requested, installed_by, installed_at, dependencies) = match &pkg.receipt {
        Some(receipt) => {
            let deps: Vec<String> = receipt
                .runtime_dependencies
                .iter()
                .map(|d| d.full_name.clone())
                .collect();

            let at = receipt
                .install_time
                .map(timestamp_to_iso)
                .unwrap_or_else(|| timestamp_now_iso());

            (receipt.installed_on_request, "brew", at, deps)
        }
        None => {
            // No receipt — conservative defaults
            (true, "unknown", timestamp_now_iso(), Vec::new())
        }
    };

    installed.add_imported(
        &pkg.name,
        &pkg.version,
        0, // revision not available from Cellar scan
        requested,
        installed_by,
        &installed_at,
        dependencies,
    );
}

fn timestamp_now_iso() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    timestamp_to_iso(now)
}
