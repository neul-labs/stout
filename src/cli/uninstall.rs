//! Uninstall command

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_install::{remove_package, scan_cellar_package, unlink_package};
use stout_state::{InstalledPackages, Paths};

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

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No packages specified");
    }

    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;
    let cask_state_path = paths.stout_dir.join("casks.json");

    for name in &args.formulas {
        // Determine if this is a formula or cask
        let is_formula_installed = installed.get(name).is_some();
        let is_cask_installed = !args.formula
            && stout_cask::InstalledCasks::load(&cask_state_path)
                .ok()
                .is_some_and(|c| c.is_installed(name));

        if args.cask {
            uninstall_cask(name, &cask_state_path, args.zap, args.dry_run).await?;
        } else if args.formula || is_formula_installed {
            uninstall_formula(name, &mut installed, &paths, args.force, args.dry_run)?;
        } else if is_cask_installed {
            uninstall_cask(name, &cask_state_path, args.zap, args.dry_run).await?;
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
    force: bool,
    dry_run: bool,
) -> Result<()> {
    let pkg = match installed.get(name) {
        Some(pkg) => pkg.clone(),
        None => {
            // Targeted sync: check if already removed from Cellar
            if let Some(cellar_pkg) = scan_cellar_package(&paths.cellar, name)? {
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

    // Dependency safety check: warn if other tracked packages depend on this one
    let dependents: Vec<String> = installed
        .iter()
        .filter(|(dep_name, dep_pkg)| {
            *dep_name != name && dep_pkg.dependencies.iter().any(|d| d == name)
        })
        .map(|(dep_name, _)| dep_name.clone())
        .collect();

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
