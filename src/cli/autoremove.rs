//! Autoremove command - uninstall packages that were only installed as dependencies
//! and are no longer needed by any installed package.

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use std::collections::HashSet;
use stout_install::unlink_package;
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Only show what would be removed without actually removing
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let mut installed = InstalledPackages::load(&paths)?;

    println!("\n{}...", style("Checking for unused dependencies").cyan());

    // Find all packages that are dependencies of other packages
    let mut needed_deps: HashSet<String> = HashSet::new();

    for name in installed.names() {
        let pkg = installed
            .get(name)
            .with_context(|| format!("failed to get package '{}' from installed", name))?;
        for dep in &pkg.dependencies {
            needed_deps.insert(dep.clone());
        }
    }

    // Find orphaned packages (installed as dependency, not explicitly requested, not needed)
    let mut orphans: Vec<String> = Vec::new();

    for name in installed.names() {
        let pkg = installed
            .get(name)
            .with_context(|| format!("failed to get package '{}' from installed", name))?;

        // Skip if explicitly requested by user
        if pkg.requested {
            continue;
        }

        // Skip if needed by another package
        if needed_deps.contains(name) {
            continue;
        }

        orphans.push(name.to_string());
    }

    if orphans.is_empty() {
        println!("\n{}", style("No unused dependencies to remove.").green());
        return Ok(());
    }

    // Sort for consistent output
    orphans.sort();

    println!(
        "\n{} {} unused dependencies:",
        style("==>").blue().bold(),
        orphans.len()
    );

    for name in &orphans {
        let pkg = installed.get(name).with_context(|| {
            format!(
                "package '{}' was in orphan list but not found in installed",
                name
            )
        })?;
        println!(
            "  {} {} {}",
            style("•").dim(),
            name,
            style(&pkg.version).dim()
        );
    }

    if args.dry_run {
        println!(
            "\n{} Would remove {} package{}",
            style("Dry run:").yellow(),
            orphans.len(),
            if orphans.len() == 1 { "" } else { "s" }
        );
        return Ok(());
    }

    println!();

    // Remove orphaned packages
    let mut removed_count = 0;
    for name in &orphans {
        let pkg = installed
            .get(name)
            .with_context(|| format!("package '{}' was in orphan list but not found", name))?;
        let install_path = paths.cellar.join(name).join(&pkg.version);

        // Unlink from prefix
        if install_path.exists() {
            if let Err(e) = unlink_package(&install_path, &paths.prefix) {
                eprintln!("  {} Failed to unlink {}: {}", style("⚠").yellow(), name, e);
                continue;
            }

            // Remove from cellar
            if let Err(e) = std::fs::remove_dir_all(&install_path) {
                eprintln!("  {} Failed to remove {}: {}", style("⚠").yellow(), name, e);
                continue;
            }

            // Remove parent dir if empty
            let parent = paths.cellar.join(name);
            if parent
                .read_dir()
                .map(|mut d| d.next().is_none())
                .unwrap_or(false)
            {
                let _ = std::fs::remove_dir(&parent);
            }
        }

        // Remove from installed state
        installed.remove(name);
        removed_count += 1;

        println!("  {} Removed {}", style("✓").green(), name);
    }

    // Save updated state
    installed.save(&paths)?;

    println!(
        "\n{} Removed {} unused dependency{}",
        style("Autoremove").green().bold(),
        removed_count,
        if removed_count == 1 { "" } else { "ies" }
    );

    Ok(())
}
