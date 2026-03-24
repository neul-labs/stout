//! Sync command — reconcile Stout's state with the Cellar

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use stout_install::cellar::{scan_cellar, scan_cellar_package, CellarPackage};
use stout_state::{InstalledPackages, Paths};

use crate::cli::import::import_cellar_package;

#[derive(ClapArgs)]
pub struct Args {
    /// Show what would change without modifying state
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Apply all changes without prompting
    #[arg(short, long)]
    pub yes: bool,

    /// Show detailed output
    #[arg(short, long)]
    pub verbose: bool,
}

/// A detected drift between state and Cellar
#[derive(Debug)]
pub enum DriftChange {
    /// Package in Cellar but not in state
    Added { name: String, version: String },
    /// Package in state but not in Cellar
    Removed { name: String, version: String },
    /// Package version in Cellar differs from state
    VersionChanged {
        name: String,
        old_version: String,
        new_version: String,
    },
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let mut installed = InstalledPackages::load(&paths)?;

    let changes = detect_drift(&installed, &paths)?;

    if changes.is_empty() {
        println!("\n{}", style("State is in sync with Cellar.").green());
        return Ok(());
    }

    print_changes(&changes);

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Prompt unless --yes
    if !args.yes {
        if !atty_is_interactive() {
            println!(
                "\n{}",
                style("Non-interactive terminal. Use --yes to apply changes.").yellow()
            );
            return Ok(());
        }
        eprint!("\n{} ", style("Apply changes? [Y/n]").bold());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input == "n" || input == "no" {
            println!("{}", style("Cancelled.").dim());
            return Ok(());
        }
    }

    let applied = apply_changes(&mut installed, &changes, &paths)?;

    installed.save(&paths)?;

    println!("\n{} {} changes", style("Synced").green().bold(), applied);

    Ok(())
}

/// Detect drift between installed state and Cellar.
pub fn detect_drift(installed: &InstalledPackages, paths: &Paths) -> Result<Vec<DriftChange>> {
    println!("\n{}...", style("Scanning Cellar").cyan());

    let cellar_packages = scan_cellar(&paths.cellar)?;
    let mut changes = Vec::new();

    // Check for packages in Cellar but not in state (added externally)
    for pkg in &cellar_packages {
        match installed.get(&pkg.name) {
            None => {
                changes.push(DriftChange::Added {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                });
            }
            Some(state_pkg) => {
                if state_pkg.version != pkg.version {
                    changes.push(DriftChange::VersionChanged {
                        name: pkg.name.clone(),
                        old_version: state_pkg.version.clone(),
                        new_version: pkg.version.clone(),
                    });
                }
            }
        }
    }

    // Check for packages in state but not in Cellar (removed externally)
    let cellar_names: std::collections::HashSet<&str> =
        cellar_packages.iter().map(|p| p.name.as_str()).collect();

    for (name, pkg) in installed.iter() {
        if !cellar_names.contains(name.as_str()) {
            changes.push(DriftChange::Removed {
                name: name.clone(),
                version: pkg.version.clone(),
            });
        }
    }

    Ok(changes)
}

fn print_changes(changes: &[DriftChange]) {
    println!("\n{}:\n", style("Changes detected").cyan());

    for change in changes {
        match change {
            DriftChange::Added { name, version } => {
                println!(
                    "  {} {} {}  {}",
                    style("+").green(),
                    name,
                    style(version).dim(),
                    style("(in Cellar, not tracked)").dim()
                );
            }
            DriftChange::Removed { name, version } => {
                println!(
                    "  {} {} {}  {}",
                    style("-").red(),
                    name,
                    style(version).dim(),
                    style("(tracked, not in Cellar)").dim()
                );
            }
            DriftChange::VersionChanged {
                name,
                old_version,
                new_version,
            } => {
                println!(
                    "  {} {} {} → {}  {}",
                    style("~").yellow(),
                    name,
                    style(old_version).dim(),
                    style(new_version).cyan(),
                    style("(version updated externally)").dim()
                );
            }
        }
    }
}

/// Apply drift changes to installed state. Returns count of changes applied.
pub fn apply_changes(
    installed: &mut InstalledPackages,
    changes: &[DriftChange],
    paths: &Paths,
) -> Result<usize> {
    let mut applied = 0;

    for change in changes {
        match change {
            DriftChange::Added { name, version } => {
                if let Some(pkg) = scan_cellar_package(&paths.cellar, name)? {
                    import_cellar_package(installed, &pkg);
                    println!(
                        "  {} Added {} {}",
                        style("✓").green(),
                        name,
                        style(version).dim()
                    );
                    applied += 1;
                }
            }
            DriftChange::Removed { name, version } => {
                installed.remove(name);
                println!("  {} Removed {} from tracking", style("✓").green(), name);
                applied += 1;
            }
            DriftChange::VersionChanged {
                name,
                old_version,
                new_version,
            } => {
                // Update the version in state
                if let Some(pkg) = installed.get(name).cloned() {
                    installed.add_imported(
                        name,
                        new_version,
                        0,
                        pkg.requested,
                        &pkg.installed_by,
                        &pkg.installed_at,
                        pkg.dependencies.clone(),
                    );
                    println!(
                        "  {} Updated {} to {}",
                        style("✓").green(),
                        name,
                        style(new_version).cyan()
                    );
                    applied += 1;
                }
            }
        }
    }

    Ok(applied)
}

/// Run sync in auto-apply mode (for use within stout update).
/// Prints informational output but doesn't prompt.
pub async fn run_auto_sync(paths: &Paths) -> Result<usize> {
    let mut installed = InstalledPackages::load(paths)?;
    let changes = detect_drift(&installed, paths)?;

    if changes.is_empty() {
        return Ok(0);
    }

    let applied = apply_changes(&mut installed, &changes, paths)?;

    if applied > 0 {
        installed.save(paths)?;
    }

    Ok(applied)
}

fn atty_is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}
