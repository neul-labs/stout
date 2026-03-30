//! List command

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use stout_state::{InstalledPackage, InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Show versions only
    #[arg(short, long)]
    pub versions: bool,

    /// Show full paths
    #[arg(short, long)]
    pub paths: bool,

    /// Filter by source: stout, brew, unknown
    #[arg(long)]
    pub source: Option<String>,

    /// Show only explicitly installed (not dependencies)
    #[arg(long)]
    pub requested: bool,

    /// Show only packages installed as dependencies
    #[arg(long)]
    pub deps: bool,

    /// Show only pinned packages
    #[arg(long)]
    pub pinned: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    if installed.count() == 0 {
        println!("\n{}", style("No packages installed.").dim());
        return Ok(());
    }

    // Collect, filter, and sort
    let mut packages: Vec<(&String, &InstalledPackage)> = installed
        .packages
        .iter()
        .filter(|(_, pkg)| {
            if let Some(ref source) = args.source {
                if pkg.installed_by != *source {
                    return false;
                }
            }
            if args.requested && !pkg.requested {
                return false;
            }
            if args.deps && pkg.requested {
                return false;
            }
            if args.pinned && !pkg.pinned {
                return false;
            }
            true
        })
        .collect();

    packages.sort_by(|a, b| a.0.cmp(b.0));

    if packages.is_empty() {
        println!("\n{}", style("No packages match the filter.").dim());
        return Ok(());
    }

    println!(
        "\n{} ({}):\n",
        style("Installed packages").cyan(),
        packages.len()
    );

    // Find max lengths for alignment
    let max_name = packages.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let max_ver = packages
        .iter()
        .map(|(_, p)| p.version.len())
        .max()
        .unwrap_or(0);
    let max_source = packages
        .iter()
        .map(|(_, p)| p.installed_by.len())
        .max()
        .unwrap_or(0)
        .max(6); // "Source" header

    // Print header
    if !args.versions && !args.paths {
        println!(
            "  {:<name_w$}  {:<ver_w$}  {:<src_w$}  {}",
            style("Name").dim(),
            style("Version").dim(),
            style("Source").dim(),
            style("Installed").dim(),
            name_w = max_name,
            ver_w = max_ver,
            src_w = max_source,
        );
        println!(
            "  {}",
            style("─".repeat(max_name + max_ver + max_source + 30)).dim(),
        );
    }

    for (name, pkg) in &packages {
        if args.paths {
            let path = paths.package_path(name, &pkg.version);
            println!(
                "  {:<width$}  {}",
                style(name).green(),
                path.display(),
                width = max_name
            );
        } else if args.versions {
            println!("  {} {}", style(name).green(), style(&pkg.version).dim());
        } else {
            let req_marker = if pkg.requested { "" } else { " (dep)" };
            println!(
                "  {:<name_w$}  {:<ver_w$}  {:<src_w$}  {}{}",
                style(name).green(),
                style(&pkg.version).dim(),
                style(&pkg.installed_by).dim(),
                style(&pkg.installed_at).dim(),
                style(req_marker).dim(),
                name_w = max_name,
                ver_w = max_ver,
                src_w = max_source,
            );
        }
    }

    // Summary with source counts
    if !args.versions && !args.paths {
        let stout_count = packages
            .iter()
            .filter(|(_, p)| p.installed_by == "stout")
            .count();
        let brew_count = packages
            .iter()
            .filter(|(_, p)| p.installed_by == "brew")
            .count();
        let unknown_count = packages.len() - stout_count - brew_count;

        let mut parts = Vec::new();
        if stout_count > 0 {
            parts.push(format!("{} stout", stout_count));
        }
        if brew_count > 0 {
            parts.push(format!("{} brew", brew_count));
        }
        if unknown_count > 0 {
            parts.push(format!("{} unknown", unknown_count));
        }

        println!(
            "\n{}: {} packages ({})",
            style("Total").dim(),
            packages.len(),
            parts.join(", ")
        );
    }

    println!();
    Ok(())
}
