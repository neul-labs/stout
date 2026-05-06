//! Uninstall command

use std::collections::HashSet;

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_index::{Database, DependencyType};
use stout_install::{remove_package, scan_cellar, scan_cellar_package, unlink_package};
use stout_state::{InstalledPackages, Paths};

use super::services;

#[derive(ClapArgs)]
pub struct Args {
    /// Packages to uninstall (formulas or casks)
    #[arg(value_name = "PACKAGES")]
    pub formulas: Vec<String>,

    /// Remove even if other packages depend on it, or remove untracked Cellar packages
    #[arg(long)]
    pub force: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,

    /// Treat all packages as casks
    #[arg(long)]
    pub cask: bool,

    /// Treat all packages as formulas
    #[arg(long, conflicts_with = "cask")]
    pub formula: bool,

    /// Remove all associated files (casks only)
    #[arg(long)]
    pub zap: bool,
}

/// Find all packages that depend on the given package.
fn find_dependents(
    name: &str,
    installed: &InstalledPackages,
    paths: &Paths,
    db: Option<&Database>,
) -> Vec<String> {
    let mut dependents = HashSet::new();

    // Source 1: stout-tracked packages
    for (dep_name, dep_pkg) in installed.iter() {
        if dep_name != name && dep_pkg.dependencies.iter().any(|d| d == name) {
            dependents.insert(dep_name.clone());
        }
    }

    // Source 2: Database (if provided)
    if let Some(database) = db {
        if let Ok(db_dependents) =
            database.get_dependents(name, DependencyType::default_dependent_types())
        {
            for dep in db_dependents {
                // Only count as a dependent if it's actually installed
                if installed.is_installed(&dep.formula) || paths.cellar.join(&dep.formula).exists()
                {
                    dependents.insert(dep.formula);
                }
            }
        }
    }

    // Source 3: Cellar receipts (fallback for untracked packages)
    if let Ok(cellar_packages) = scan_cellar(&paths.cellar) {
        for pkg in &cellar_packages {
            if pkg.name == name {
                continue;
            }
            if let Some(receipt) = &pkg.receipt {
                if receipt
                    .runtime_dependencies
                    .iter()
                    .any(|d| d.full_name == name)
                {
                    dependents.insert(pkg.name.clone());
                }
            }
        }
    }

    let mut result: Vec<String> = dependents.into_iter().collect();
    result.sort();
    result
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No packages specified");
    }

    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;
    let cask_state_path = paths.stout_dir.join("casks.json");
    let db = Database::open(paths.index_db()).ok();

    for name in &args.formulas {
        // Determine if this is a formula or cask
        let is_formula_installed = installed.get(name).is_some();
        let is_cask_installed = !args.formula
            && stout_cask::InstalledCasks::load(&cask_state_path)
                .ok()
                .is_some_and(|c| c.is_installed(name));

        if args.cask {
            uninstall_cask(name, &cask_state_path, &paths, args.zap, args.dry_run).await?;
        } else if args.formula || is_formula_installed {
            uninstall_formula(
                name,
                &mut installed,
                &paths,
                db.as_ref(),
                args.force,
                args.dry_run,
            )?;
        } else if is_cask_installed {
            uninstall_cask(name, &cask_state_path, &paths, args.zap, args.dry_run).await?;
        } else {
            eprintln!("{} {} is not installed", style("error:").red().bold(), name);
        }
    }

    if !args.dry_run {
        installed.save(&paths)?;
    }

    Ok(())
}

fn uninstall_formula(
    name: &str,
    installed: &mut InstalledPackages,
    paths: &Paths,
    db: Option<&Database>,
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let pkg = match installed.get(name) {
        Some(pkg) => pkg.clone(),
        None => {
            // Targeted sync: check if already removed from Cellar
            if let Some(cellar_pkg) = scan_cellar_package(&paths.cellar, name)? {
                // Check dependents even for untracked packages
                let dependents = find_dependents(name, installed, paths, db);
                if !dependents.is_empty() && !force {
                    eprintln!(
                        "{} {} is a dependency of: {}",
                        style("error:").red().bold(),
                        name,
                        dependents.join(", ")
                    );
                    eprintln!("  {}", style("Use --force to remove anyway").dim());
                    return Ok(());
                }

                if !dependents.is_empty() {
                    println!(
                        "  {} {} is a dependency of: {}",
                        style("⚠").yellow(),
                        name,
                        dependents.join(", ")
                    );
                }

                if force {
                    // Package in Cellar but not tracked — force remove
                    if dry_run {
                        println!(
                            "Would uninstall {} {} {}",
                            style(name).green(),
                            style(&cellar_pkg.version).dim(),
                            style("(untracked, force)").yellow()
                        );
                        return Ok(());
                    }

                    println!(
                        "{}...",
                        style(format!("Uninstalling {} {}", name, cellar_pkg.version)).cyan()
                    );

                    let install_path = paths.package_path(name, &cellar_pkg.version);
                    services::stop_package_service(name, &install_path);
                    let _ = unlink_package(&install_path, &paths.prefix);
                    let _ = remove_package(&paths.cellar, name, &cellar_pkg.version);

                    println!(
                        "  {} {} {} {}",
                        style("⚠").yellow(),
                        name,
                        cellar_pkg.version,
                        style("(was not tracked by stout, removed from Cellar)").yellow()
                    );
                    return Ok(());
                } else {
                    eprintln!(
                        "{} {} is not tracked by stout (use --force to remove from Cellar)",
                        style("error:").red().bold(),
                        name
                    );
                    return Ok(());
                }
            }

            eprintln!("{} {} is not installed", style("error:").red().bold(), name);
            return Ok(());
        }
    };

    // Targeted sync: check if already removed from Cellar externally
    let install_path = paths.package_path(name, &pkg.version);
    if !install_path.exists() {
        // Package was removed externally — clean up state
        installed.remove(name);
        println!(
            "  {} {} {} {}",
            style("✓").green(),
            name,
            pkg.version,
            style("(already removed from Cellar, cleaned up state)").dim()
        );
        return Ok(());
    }

    // Dependency safety check: find all packages that depend on this one
    let dependents = find_dependents(name, installed, paths, db);

    if !dependents.is_empty() && !force {
        eprintln!(
            "{} {} is a dependency of: {}",
            style("error:").red().bold(),
            name,
            dependents.join(", ")
        );
        eprintln!("  {}", style("Use --force to remove anyway").dim());
        return Ok(());
    }

    if !dependents.is_empty() {
        println!(
            "  {} {} is a dependency of: {}",
            style("⚠").yellow(),
            name,
            dependents.join(", ")
        );
    }

    if dry_run {
        println!(
            "Would uninstall {} {}",
            style(name).green(),
            style(&pkg.version).dim()
        );
        return Ok(());
    }

    println!(
        "{}...",
        style(format!("Uninstalling {} {}", name, pkg.version)).cyan()
    );

    // Stop service if one is running
    services::stop_package_service(name, &install_path);

    // Unlink
    unlink_package(&install_path, &paths.prefix)?;

    // Remove from Cellar
    remove_package(&paths.cellar, name, &pkg.version)?;

    // Update state
    installed.remove(name);

    println!("  {} {} {}", style("✓").green(), name, pkg.version);
    Ok(())
}

async fn uninstall_cask(
    name: &str,
    cask_state_path: &std::path::Path,
    paths: &stout_state::Paths,
    zap: bool,
    dry_run: bool,
) -> Result<()> {
    let installed_casks = stout_cask::InstalledCasks::load(cask_state_path)?;

    match installed_casks.get(name) {
        Some(cask) => {
            if dry_run {
                println!(
                    "Would uninstall {} {}{}",
                    style(name).green(),
                    style(&cask.version).dim(),
                    if zap {
                        style(" (zap)").yellow().to_string()
                    } else {
                        String::new()
                    }
                );
                return Ok(());
            }

            println!(
                "{}...",
                style(format!("Uninstalling {} {}", name, cask.version)).cyan()
            );

            stout_cask::uninstall_cask(name, cask_state_path, zap).await?;

            // Remove from Caskroom so sync doesn't see a stale entry
            if let Err(e) = stout_install::cask_scan::unregister_cask_from_caskroom(
                &paths.prefix,
                name,
                &cask.version,
            ) {
                tracing::debug!("Failed to unregister {} from Caskroom: {}", name, e);
            }

            println!("  {} {} {}", style("✓").green(), name, cask.version);
        }
        None => {
            eprintln!(
                "{} cask {} is not installed",
                style("error:").red().bold(),
                name
            );
        }
    }

    Ok(())
}
