//! Uninstall command

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_install::{remove_package, scan_cellar_package, unlink_package};
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to uninstall
    pub formulas: Vec<String>,

    /// Remove even if other packages depend on it, or remove untracked Cellar packages
    #[arg(long)]
    pub force: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No formulas specified");
    }

    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;

    for name in &args.formulas {
        let pkg = match installed.get(name) {
            Some(pkg) => pkg.clone(),
            None => {
                // Targeted sync: check if already removed from Cellar
                if let Some(cellar_pkg) = scan_cellar_package(&paths.cellar, name)? {
                    if args.force {
                        // Package in Cellar but not tracked — force remove
                        if args.dry_run {
                            println!(
                                "Would uninstall {} {} {}",
                                style(name).green(),
                                style(&cellar_pkg.version).dim(),
                                style("(untracked, force)").yellow()
                            );
                            continue;
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
                        continue;
                    } else {
                        eprintln!(
                            "{} {} is not tracked by stout (use --force to remove from Cellar)",
                            style("error:").red().bold(),
                            name
                        );
                        continue;
                    }
                }

                eprintln!("{} {} is not installed", style("error:").red().bold(), name);
                continue;
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
            continue;
        }

        // Dependency safety check: warn if other tracked packages depend on this one
        let dependents: Vec<String> = installed
            .iter()
            .filter(|(dep_name, dep_pkg)| {
                *dep_name != name && dep_pkg.dependencies.iter().any(|d| d == name)
            })
            .map(|(dep_name, _)| dep_name.clone())
            .collect();

        if !dependents.is_empty() && !args.force {
            eprintln!(
                "{} {} is a dependency of: {}",
                style("error:").red().bold(),
                name,
                dependents.join(", ")
            );
            eprintln!("  {}", style("Use --force to remove anyway").dim());
            continue;
        }

        if !dependents.is_empty() {
            println!(
                "  {} {} is a dependency of: {}",
                style("⚠").yellow(),
                name,
                dependents.join(", ")
            );
        }

        if args.dry_run {
            println!(
                "Would uninstall {} {}",
                style(name).green(),
                style(&pkg.version).dim()
            );
            continue;
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
    }

    if !args.dry_run {
        installed.save(&paths)?;
    }

    Ok(())
}
