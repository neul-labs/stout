//! Uninstall command

use anyhow::{bail, Result};
use brewx_install::{remove_package, unlink_package};
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to uninstall
    pub formulas: Vec<String>,

    /// Remove even if other packages depend on it
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
                eprintln!(
                    "{} {} is not installed",
                    style("error:").red().bold(),
                    name
                );
                continue;
            }
        };

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

        let install_path = paths.package_path(name, &pkg.version);

        // Unlink
        unlink_package(&install_path, &paths.prefix)?;

        // Remove from Cellar
        brewx_install::remove_package(&paths.cellar, name, &pkg.version)?;

        // Update state
        installed.remove(name);

        println!("  {} {} {}", style("✓").green(), name, pkg.version);
    }

    if !args.dry_run {
        installed.save(&paths)?;
    }

    Ok(())
}
