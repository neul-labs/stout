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

    /// Show only casks
    #[arg(long)]
    pub cask: bool,

    /// Show only formulas
    #[arg(long, conflicts_with = "cask")]
    pub formula: bool,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    let show_formulas = !args.cask;
    let show_casks = !args.formula;

    if show_formulas {
        list_formulas(&args, &paths)?;
    }

    if show_casks {
        list_casks(&args, &paths)?;
    }

    Ok(())
}

fn list_formulas(args: &Args, paths: &Paths) -> Result<()> {
    let installed = InstalledPackages::load(paths)?;

    if installed.count() == 0 {
        if args.cask {
            return Ok(()); // Skip silently when only showing casks
        }
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
        if !args.cask {
            println!("\n{}", style("No packages match the filter.").dim());
        }
        return Ok(());
    }

    println!(
        "\n{} ({}):\n",
        style("Installed formulas").cyan(),
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

fn list_casks(args: &Args, paths: &Paths) -> Result<()> {
    let cask_state_path = paths.stout_dir.join("casks.json");
    let installed_casks = stout_cask::InstalledCasks::load(&cask_state_path)?;

    if installed_casks.count() == 0 {
        if args.formula {
            return Ok(()); // Skip silently when only showing formulas
        }
        println!("\n{}", style("No casks installed.").dim());
        return Ok(());
    }

    let mut casks: Vec<_> = installed_casks.iter().collect();
    casks.sort_by(|a, b| a.0.cmp(b.0));

    if args.json {
        let cask_list: Vec<serde_json::Value> = casks
            .iter()
            .map(|(token, cask)| {
                serde_json::json!({
                    "token": token,
                    "version": cask.version,
                    "installed_at": cask.installed_at,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&cask_list)?);
        return Ok(());
    }

    println!(
        "\n{} ({}):\n",
        style("Installed casks").magenta(),
        casks.len()
    );

    let max_name = casks.iter().map(|(n, _)| n.len()).max().unwrap_or(0);

    for (token, cask) in &casks {
        if args.versions {
            println!(
                "  {} {}",
                style(token).magenta(),
                style(&cask.version).dim()
            );
        } else {
            println!(
                "  {:<name_w$}  {}  {}",
                style(token).magenta(),
                style(&cask.version).dim(),
                style(&cask.installed_at).dim(),
                name_w = max_name,
            );
        }
    }

    println!("\n{}: {} casks", style("Total").dim(), casks.len());

    println!();
    Ok(())
}
