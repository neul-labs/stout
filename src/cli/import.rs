//! Import command — import existing Homebrew packages into Stout's state

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use stout_cask::InstalledCasks;
use stout_install::cask_scan::scan_caskroom;
use stout_install::cellar::{scan_cellar, scan_cellar_package, timestamp_to_iso, CellarPackage};
use stout_install::relocate_bottle;
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

    println!("\n{}...", style("Scanning Homebrew").cyan());

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

        // Relocate Homebrew placeholders (@@HOMEBREW_PREFIX@@, etc.)
        match relocate_bottle(&pkg.path, &paths.prefix) {
            Ok(count) if count > 0 && args.verbose => {
                println!(
                    "  {} {} {} {}",
                    style("✓").green(),
                    pkg.name,
                    style(&pkg.version).dim(),
                    style(format!("(relocated {} files)", count)).dim()
                );
            }
            Ok(_) => {
                println!(
                    "  {} {} {}",
                    style("✓").green(),
                    pkg.name,
                    style(&pkg.version).dim()
                );
            }
            Err(e) => {
                eprintln!(
                    "  {} {} {} {}",
                    style("✓").green(),
                    pkg.name,
                    style(&pkg.version).dim(),
                    style(format!("(relocation warning: {})", e)).yellow()
                );
            }
        }

        imported += 1;
    }

    if !args.dry_run && imported > 0 {
        installed.save(&paths)?;
    }

    println!(
        "\n{} {} packages ({} already tracked)",
        if args.dry_run {
            style("Would import").yellow()
        } else {
            style("Imported").green().bold()
        },
        imported,
        skipped,
    );

    // Import casks (only when importing all, not specific packages)
    if args.packages.is_empty() {
        import_brew_casks(&paths, args.dry_run, args.overwrite, args.verbose)?;
    }

    Ok(())
}

/// Import installed Homebrew casks into Stout's cask state.
fn import_brew_casks(paths: &Paths, dry_run: bool, overwrite: bool, verbose: bool) -> Result<()> {
    let brew_casks = scan_caskroom(&paths.prefix).unwrap_or_default();
    if brew_casks.is_empty() {
        return Ok(());
    }

    let cask_state_path = paths.stout_dir.join("casks.json");
    let mut cask_state = InstalledCasks::load(&cask_state_path).unwrap_or_default();

    println!("\n{}...", style("Scanning Homebrew casks").cyan());

    let mut cask_imported = 0u32;
    let mut cask_skipped = 0u32;

    for cask in &brew_casks {
        if cask_state.is_installed(&cask.token) && !overwrite {
            if verbose {
                println!(
                    "  {} {} {}",
                    style("⊘").dim(),
                    cask.token,
                    style("(already tracked)").dim()
                );
            }
            cask_skipped += 1;
            continue;
        }

        if dry_run {
            println!("  {} {}", style("✓").green(), cask.token);
            cask_imported += 1;
            continue;
        }

        let timestamp = timestamp_now_iso();
        let imported_cask = stout_cask::InstalledCask {
            version: cask.version.clone().unwrap_or_else(|| "unknown".to_string()),
            installed_at: timestamp,
            artifact_path: std::path::PathBuf::from(""),
            auto_updates: false,
            artifacts: Vec::new(),
        };
        cask_state.add(&cask.token, imported_cask);
        cask_imported += 1;

        println!("  {} {}", style("✓").green(), cask.token);
    }

    if !dry_run && cask_imported > 0 {
        cask_state.save(&cask_state_path)?;
    }

    println!(
        "\n{} {} casks ({} already tracked)",
        if dry_run {
            style("Would import").yellow()
        } else {
            style("Imported").green().bold()
        },
        cask_imported,
        cask_skipped,
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

pub fn timestamp_now_iso() -> String {
    jiff::Timestamp::now()
        .strftime("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}
