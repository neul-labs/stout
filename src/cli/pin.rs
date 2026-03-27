//! Pin command - prevent packages from being upgraded

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to pin
    pub formulas: Vec<String>,
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        // List pinned packages
        let paths = Paths::default();
        let installed = InstalledPackages::load(&paths)?;

        let pinned: Vec<_> = installed.pinned().collect();

        if pinned.is_empty() {
            println!("{}", style("No pinned packages.").dim());
        } else {
            println!("{} Pinned packages:", style("==>").blue().bold());
            for (name, pkg) in pinned {
                println!(
                    "  {} {} {}",
                    style("•").dim(),
                    name,
                    style(&pkg.version).dim()
                );
            }
        }
        return Ok(());
    }

    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;

    let mut pinned_count = 0;

    for name in &args.formulas {
        if !installed.is_installed(name) {
            eprintln!("{} {} is not installed", style("Warning:").yellow(), name);
            continue;
        }

        if installed.is_pinned(name) {
            println!("{} {} is already pinned", style("•").dim(), name);
            continue;
        }

        installed.pin(name);
        pinned_count += 1;
        println!("{} Pinned {}", style("✓").green(), name);
    }

    if pinned_count > 0 {
        installed.save(&paths)?;
    }

    Ok(())
}
