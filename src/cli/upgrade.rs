//! Upgrade command

use anyhow::{Context, Result};
use brewx_index::Database;
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Specific formulas to upgrade (all if none specified)
    pub formulas: Vec<String>,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    let installed = InstalledPackages::load(&paths)?;

    println!("\n{}...", style("Checking for updates").cyan());

    // Find upgradable packages
    let mut upgradable = Vec::new();

    let packages_to_check: Vec<_> = if args.formulas.is_empty() {
        installed.names().cloned().collect()
    } else {
        args.formulas.clone()
    };

    for name in packages_to_check {
        let pkg = match installed.get(&name) {
            Some(pkg) => pkg,
            None => continue,
        };

        let info = match db.get_formula(&name)? {
            Some(info) => info,
            None => continue,
        };

        if info.version != pkg.version {
            upgradable.push((name.clone(), pkg.version.clone(), info.version.clone()));
        }
    }

    if upgradable.is_empty() {
        println!("\n{}", style("All packages are up to date.").green());
        return Ok(());
    }

    // Show upgradable
    println!("\n{} packages can be upgraded:\n", upgradable.len());

    let max_name = upgradable.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);
    let max_old = upgradable.iter().map(|(_, o, _)| o.len()).max().unwrap_or(0);

    println!(
        "  {:<name_w$}  {:<old_w$}     {}",
        style("Package").dim(),
        style("Current").dim(),
        style("Latest").dim(),
        name_w = max_name,
        old_w = max_old,
    );
    println!(
        "  {:<name_w$}",
        style("─".repeat(max_name + max_old + 15)).dim(),
        name_w = 0,
    );

    for (name, old_ver, new_ver) in &upgradable {
        println!(
            "  {:<name_w$}  {:<old_w$}  →  {}",
            style(name).green(),
            old_ver,
            style(new_ver).cyan(),
            name_w = max_name,
            old_w = max_old,
        );
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // TODO: Actually perform upgrades (similar to install flow)
    println!(
        "\n{}",
        style("Run 'brewx install <package>' to upgrade individual packages").dim()
    );

    Ok(())
}
