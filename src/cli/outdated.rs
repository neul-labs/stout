//! Outdated command - list packages with available updates

use anyhow::{Context, Result};
use stout_audit::compare_versions;
use stout_index::Database;
use stout_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;
use std::cmp::Ordering;

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

    /// Check HEAD packages for updates
    #[arg(long = "fetch-HEAD")]
    pub fetch_head: bool,
}

/// Information about an outdated package
#[derive(Debug, serde::Serialize)]
struct OutdatedPackage {
    name: String,
    installed_version: String,
    current_version: String,
    pinned: bool,
}

/// Information about a HEAD package with updates
#[derive(Debug, serde::Serialize)]
struct HeadUpdate {
    name: String,
    installed_sha: String,
    latest_sha: String,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;
    let config = Config::load(&paths)?;

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
    let mut head_updates: Vec<HeadUpdate> = Vec::new();

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
        if pkg.is_head_install() {
            continue;
        }

        // Look up current version in index
        if let Ok(Some(info)) = db.get_formula(&name) {
            // Only mark as outdated if installed version is strictly less than current
            if compare_versions(&pkg.version, &info.version) == Ordering::Less {
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

    // Check HEAD packages for updates if --fetch-HEAD is specified
    if args.fetch_head {
        let sync = IndexSync::with_security_policy(
            Some(&config.index.base_url),
            &paths.stout_dir,
            config.security.to_security_policy(),
        )?;

        for (name, pkg) in installed.iter() {
            if !pkg.is_head_install() {
                continue;
            }

            let formula = match sync.fetch_formula_cached(name, None).await {
                Ok(f) => f,
                Err(_) => continue,
            };

            let head_url = match &formula.urls.head {
                Some(url) => url,
                None => continue,
            };

            // Get remote HEAD SHA (without cloning)
            let remote_sha = get_remote_head_sha(&head_url.url, &head_url.branch).ok();

            if let (Some(current), Some(remote)) = (&pkg.head_sha, remote_sha) {
                if current != &remote {
                    let short_remote: String = remote.chars().take(7).collect();
                    head_updates.push(HeadUpdate {
                        name: name.clone(),
                        installed_sha: pkg.short_sha().unwrap_or("?").to_string(),
                        latest_sha: short_remote,
                    });
                }
            }
        }
    }

    if args.json {
        // JSON output
        let output = serde_json::json!({
            "outdated": outdated,
            "head_updates": head_updates,
        });
        let json = serde_json::to_string_pretty(&output)?;
        println!("{}", json);
    } else if outdated.is_empty() && head_updates.is_empty() {
        // No outdated packages
        if args.formulas.is_empty() {
            println!("{}", style("All packages are up to date.").green());
        } else {
            println!("{}", style("Specified packages are up to date.").green());
        }
    } else {
        // Human-readable output for outdated packages
        if !outdated.is_empty() {
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

        // Human-readable output for HEAD updates
        if !head_updates.is_empty() {
            if !outdated.is_empty() {
                println!();
            }

            for update in &head_updates {
                println!(
                    "{} {} (HEAD) {} → {}",
                    style("~").yellow(),
                    style(&update.name).cyan(),
                    style(&update.installed_sha).dim(),
                    style(&update.latest_sha).green()
                );
            }

            println!(
                "\n{} {} HEAD package{} have updates",
                style("==>").blue().bold(),
                head_updates.len(),
                if head_updates.len() == 1 { "" } else { "s" }
            );

            println!(
                "\n  {}",
                style("Use 'stout reinstall <package>' to update HEAD packages").dim()
            );
        }
    }

    Ok(())
}

/// Get remote HEAD SHA without cloning (uses git ls-remote)
fn get_remote_head_sha(url: &str, branch: &Option<String>) -> Result<String> {
    let branch = branch.as_deref().unwrap_or("HEAD");

    let output = std::process::Command::new("git")
        .args(["ls-remote", url, branch])
        .output()
        .context("git ls-remote failed")?;

    if !output.status.success() {
        anyhow::bail!("git ls-remote returned non-zero exit code");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sha = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No SHA in git ls-remote output"))?;

    Ok(sha.to_string())
}
