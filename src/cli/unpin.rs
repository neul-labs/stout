//! Unpin command - allow packages to be upgraded again

use anyhow::{bail, Result};
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to unpin
    pub formulas: Vec<String>,
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No formulas specified. Use 'brewx pin' to see pinned packages.");
    }

    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;

    let mut unpinned_count = 0;

    for name in &args.formulas {
        if !installed.is_installed(name) {
            eprintln!(
                "{} {} is not installed",
                style("Warning:").yellow(),
                name
            );
            continue;
        }

        if !installed.is_pinned(name) {
            println!("{} {} is not pinned", style("•").dim(), name);
            continue;
        }

        installed.unpin(name);
        unpinned_count += 1;
        println!("{} Unpinned {}", style("✓").green(), name);
    }

    if unpinned_count > 0 {
        installed.save(&paths)?;
    }

    Ok(())
}
