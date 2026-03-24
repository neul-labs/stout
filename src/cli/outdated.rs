//! Outdated command - list packages with available updates

use anyhow::{Context, Result};
use stout_index::Database;
use stout_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Only check specific formulas
    pub formulas: Vec<String>,

    /// Show detailed version information
    #[arg(long, short = 'v')]
    pub verbose: bool,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Only list outdated formulas (not casks)
    #[arg(long)]
    pub formula: bool,

    /// Only list outdated casks
    #[arg(long)]
    pub cask: bool,

    /// List packages that would be upgraded with `stout upgrade`
    #[arg(long)]
    pub greedy: bool,
}

/// Information about an outdated package
#[derive(Debug, serde::Serialize)]
struct OutdatedPackage {
    name: String,
    installed_version: String,
    current_version: String,
    pinned: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'stout update' first.",
            style("error:").red().bold()
        );
        std::process::exit(1);
    }

    let mut outdated: Vec<OutdatedPackage> = Vec::new();

    // Get list of packages to check
    let packages_to_check: Vec<String> = if args.formulas.is_empty() {
        installed.names().map(|s| s.to_string()).collect()
    } else {
        args.formulas.clone()
    };

    for name in packages_to_check {
        let pkg = match installed.get(&name) {
            Some(p) => p,
            None => continue,
        };

        // Skip HEAD formulas - they are not compared against stable versions
        if pkg.version.starts_with("HEAD") {
            continue;
        }

        // Look up current version in index
        if let Ok(Some(info)) = db.get_formula(&name) {
            if info.version != pkg.version {
                outdated.push(OutdatedPackage {
                    name: name.clone(),
                    installed_version: pkg.version.clone(),
                    current_version: info.version,
                    pinned: pkg.pinned,
                });
            }
        }
    }

    // Filter pinned packages unless --greedy
    if !args.greedy {
        outdated.retain(|p| !p.pinned);
    }

    if args.json {
        // JSON output
        let json = serde_json::to_string_pretty(&outdated)?;
        println!("{}", json);
    } else if outdated.is_empty() {
        // No outdated packages
        if args.formulas.is_empty() {
            println!("{}", style("All packages are up to date.").green());
        } else {
            println!("{}", style("Specified packages are up to date.").green());
        }
    } else {
        // Human-readable output
        for pkg in &outdated {
            if args.verbose {
                println!(
                    "{} {} -> {}{}",
                    style(&pkg.name).cyan(),
                    style(&pkg.installed_version).yellow(),
                    style(&pkg.current_version).green(),
                    if pkg.pinned {
                        style(" [pinned]").dim().to_string()
                    } else {
                        String::new()
                    }
                );
            } else {
                print!("{}", pkg.name);
                if pkg.pinned {
                    print!(" {}", style("[pinned]").dim());
                }
                println!();
            }
        }

        println!(
            "\n{} {} outdated package{}",
            style("==>").blue().bold(),
            outdated.len(),
            if outdated.len() == 1 { "" } else { "s" }
        );
    }

    Ok(())
}
