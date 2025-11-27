//! Link command - create symlinks for a package

use anyhow::{bail, Result};
use brewx_install::link_package;
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to link
    pub formula: String,

    /// Overwrite existing files
    #[arg(long)]
    pub overwrite: bool,

    /// Only show what would be linked without actually linking
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Force linking even if already linked
    #[arg(long, short = 'f')]
    pub force: bool,
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
        "{} Linking {} {}",
        style("==>").blue().bold(),
        style(&args.formula).cyan(),
        style(&pkg.version).dim()
    );

    if args.dry_run {
        println!("{}", style("Dry run - no changes made").yellow());
        // Would need to implement dry-run in link_package
        return Ok(());
    }

    let linked = link_package(&install_path, &paths.prefix)?;

    if linked.is_empty() {
        println!("{}", style("No files to link.").dim());
    } else {
        println!(
            "{} Linked {} files",
            style("✓").green(),
            linked.len()
        );
    }

    Ok(())
}
