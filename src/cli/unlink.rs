//! Unlink command - remove symlinks for a package without uninstalling

use anyhow::{bail, Result};
use brewx_install::unlink_package;
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to unlink
    pub formula: String,

    /// Only show what would be unlinked without actually unlinking
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let pkg = installed
        .get(&args.formula)
        .ok_or_else(|| anyhow::anyhow!("Formula '{}' is not installed", args.formula))?;

    let install_path = paths.cellar.join(&args.formula).join(&pkg.version);

    if !install_path.exists() {
        bail!(
            "Cellar path does not exist: {}",
            install_path.display()
        );
    }

    println!(
        "{} Unlinking {} {}",
        style("==>").blue().bold(),
        style(&args.formula).cyan(),
        style(&pkg.version).dim()
    );

    if args.dry_run {
        println!("{}", style("Dry run - no changes made").yellow());
        return Ok(());
    }

    let unlinked = unlink_package(&install_path, &paths.prefix)?;

    if unlinked.is_empty() {
        println!("{}", style("No files were unlinked.").dim());
    } else {
        println!(
            "{} Unlinked {} files",
            style("✓").green(),
            unlinked.len()
        );
    }

    Ok(())
}
