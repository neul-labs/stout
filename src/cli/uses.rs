//! Uses command - show packages that depend on a given package

use anyhow::{bail, Context, Result};
use stout_index::Database;
use stout_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to find dependents for
    pub formula: String,

    /// Only show installed packages that use this formula
    #[arg(long)]
    pub installed: bool,

    /// Include packages where this is a build dependency
    #[arg(long)]
    pub include_build: bool,

    /// Include packages where this is a test dependency
    #[arg(long)]
    pub include_test: bool,

    /// Include packages where this is an optional dependency
    #[arg(long)]
    pub include_optional: bool,

    /// Recursively find all dependents
    #[arg(long, short = 'r')]
    pub recursive: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        bail!("Index not initialized. Run 'stout update' first.");
    }

    // Verify the formula exists
    if db.get_formula(&args.formula)?.is_none() {
        bail!("Formula '{}' not found", args.formula);
    }

    let installed = InstalledPackages::load(&paths)?;

    // Find all packages that depend on this formula
    let mut dependents: Vec<String> = Vec::new();

    if args.installed {
        // Check only installed packages
        for name in installed.names() {
            let pkg = installed.get(name)
                .with_context(|| format!("package '{}' is in installed list but not found", name))?;
            if pkg.dependencies.contains(&args.formula) {
                dependents.push(name.to_string());
            }
        }
    } else {
        // Check all formulas in the index
        // This would require iterating all formulas - for now we'll check installed
        // and note this limitation
        for name in installed.names() {
            let pkg = installed.get(name)
                .with_context(|| format!("package '{}' is in installed list but not found", name))?;
            if pkg.dependencies.contains(&args.formula) {
                dependents.push(name.to_string());
            }
        }

        // Note: A full implementation would scan all formulas in the database
        // which requires a different query pattern
    }

    dependents.sort();

    if dependents.is_empty() {
        println!(
            "No {} packages depend on {}",
            if args.installed { "installed" } else { "" },
            style(&args.formula).cyan()
        );
        return Ok(());
    }

    println!(
        "{} {} {} package{} that {} {}:",
        style("==>").blue().bold(),
        dependents.len(),
        if args.installed { "installed" } else { "" },
        if dependents.len() == 1 { "" } else { "s" },
        if dependents.len() == 1 { "uses" } else { "use" },
        style(&args.formula).cyan()
    );

    for dep in &dependents {
        let version = installed
            .get(dep)
            .map(|p| p.version.as_str())
            .unwrap_or_default();
        println!(
            "  {} {} {}",
            style("•").dim(),
            dep,
            style(version).dim()
        );
    }

    Ok(())
}
