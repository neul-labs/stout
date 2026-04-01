//! Sync command — reconcile Stout's state with Homebrew (Cellar + Caskroom)

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use stout_cask::InstalledCasks;
use stout_install::cask_scan::scan_caskroom;
use stout_install::cellar::{scan_cellar, scan_cellar_package};
use stout_state::{InstalledPackages, Paths};

use crate::cli::import::{import_cellar_package, timestamp_now_iso};

#[derive(ClapArgs)]
pub struct Args {
    /// Show what would change without modifying state
    #[arg(short = 'n', long)]
    pub dry_run: bool,

    /// Apply all changes without prompting
    #[arg(short, long)]
    pub yes: bool,

    /// Show detailed output
    #[arg(short, long)]
    pub verbose: bool,
}

/// A detected drift between state and Homebrew
#[derive(Debug)]
pub enum DriftChange {
    /// Formula in Cellar but not in state
    FormulaAdded { name: String, version: String },
    /// Formula in state but not in Cellar
    FormulaRemoved { name: String, version: String },
    /// Formula version in Cellar differs from state
    FormulaVersionChanged {
        name: String,
        old_version: String,
        new_version: String,
    },
    /// Cask installed but not in state
    CaskAdded {
        token: String,
        version: Option<String>,
    },
    /// Cask in state but not installed
    CaskRemoved { token: String },
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let mut installed = InstalledPackages::load(&paths)?;

    // First-run import prompt
    super::first_run::check_first_run_import(&mut installed, &paths)?;

    println!("\n{}...", style("Scanning Homebrew").cyan());
    let changes = detect_drift(&installed, &paths)?;

    if changes.is_empty() {
        println!("\n{}", style("State is in sync with Homebrew.").green());
        return Ok(());
    }

    print_changes(&changes);

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Prompt unless --yes
    if !args.yes {
        if !crate::output::is_interactive() {
            println!(
                "\n{}",
                style("Non-interactive terminal. Use --yes to apply changes.").yellow()
            );
            return Ok(());
        }
        eprint!("\n{} ", style("Apply changes? [Y/n]").bold());
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();
        if input == "n" || input == "no" {
            println!("{}", style("Cancelled.").dim());
            return Ok(());
        }
    }

    let applied = apply_changes(&mut installed, &changes, &paths, false)?;

    installed.save(&paths)?;

    println!("\n{} {} changes", style("Synced").green().bold(), applied);

    Ok(())
}

/// Detect drift between installed state and Homebrew.
pub fn detect_drift(installed: &InstalledPackages, paths: &Paths) -> Result<Vec<DriftChange>> {
    let mut changes = Vec::new();

    // Scan Cellar (formulas)
    let cellar_packages = scan_cellar(&paths.cellar)?;

    // Check for formulas in Cellar but not in state (added externally)
    for pkg in &cellar_packages {
        match installed.get(&pkg.name) {
            None => {
                changes.push(DriftChange::FormulaAdded {
                    name: pkg.name.clone(),
                    version: pkg.version.clone(),
                });
            }
            Some(state_pkg) => {
                if state_pkg.version != pkg.version {
                    changes.push(DriftChange::FormulaVersionChanged {
                        name: pkg.name.clone(),
                        old_version: state_pkg.version.clone(),
                        new_version: pkg.version.clone(),
                    });
                }
            }
        }
    }

    // Check for formulas in state but not in Cellar (removed externally)
    let cellar_names: std::collections::HashSet<&str> =
        cellar_packages.iter().map(|p| p.name.as_str()).collect();

    for (name, pkg) in installed.iter() {
        if !cellar_names.contains(name.as_str()) {
            changes.push(DriftChange::FormulaRemoved {
                name: name.clone(),
                version: pkg.version.clone(),
            });
        }
    }

    // Scan Casks
    let brew_casks = scan_caskroom(&paths.prefix).unwrap_or_default();
    let cask_state_path = paths.stout_dir.join("casks.json");
    let cask_state = InstalledCasks::load(&cask_state_path).unwrap_or_default();

    // Check for casks installed but not in state
    for cask in &brew_casks {
        if !cask_state.is_installed(&cask.token) {
            changes.push(DriftChange::CaskAdded {
                token: cask.token.clone(),
                version: cask.version.clone(),
            });
        }
    }

    // Check for casks in state but not installed
    let installed_cask_tokens: std::collections::HashSet<&str> =
        brew_casks.iter().map(|c| c.token.as_str()).collect();
    for token in cask_state.tokens() {
        if !installed_cask_tokens.contains(token.as_str()) {
            changes.push(DriftChange::CaskRemoved {
                token: token.clone(),
            });
        }
    }

    Ok(changes)
}

fn print_changes(changes: &[DriftChange]) {
    println!("\n{}:\n", style("Changes detected").cyan());

    for change in changes {
        match change {
            DriftChange::FormulaAdded { name, version } => {
                println!(
                    "  {} {} {}  {}",
                    style("+").green(),
                    name,
                    style(version).dim(),
                    style("(not tracked)").dim()
                );
            }
            DriftChange::FormulaRemoved { name, version } => {
                println!(
                    "  {} {} {}  {}",
                    style("-").red(),
                    name,
                    style(version).dim(),
                    style("(tracked, not installed)").dim()
                );
            }
            DriftChange::FormulaVersionChanged {
                name,
                old_version,
                new_version,
            } => {
                println!(
                    "  {} {} {} → {}  {}",
                    style("~").yellow(),
                    name,
                    style(old_version).dim(),
                    style(new_version).cyan(),
                    style("(version updated externally)").dim()
                );
            }
            DriftChange::CaskAdded { token, .. } => {
                println!(
                    "  {} {}  {}",
                    style("+").green(),
                    token,
                    style("(not tracked)").dim()
                );
            }
            DriftChange::CaskRemoved { token } => {
                println!(
                    "  {} {}  {}",
                    style("-").red(),
                    token,
                    style("(tracked, not installed)").dim()
                );
            }
        }
    }
}

/// Apply drift changes to installed state. Returns count of changes applied.
///
/// When `quiet` is true, no output is printed (caller controls formatting).
pub fn apply_changes(
    installed: &mut InstalledPackages,
    changes: &[DriftChange],
    paths: &Paths,
    quiet: bool,
) -> Result<usize> {
    let mut applied = 0;

    let cask_state_path = paths.stout_dir.join("casks.json");
    let mut cask_state = InstalledCasks::load(&cask_state_path).unwrap_or_default();

    for change in changes {
        match change {
            DriftChange::FormulaAdded { name, version } => {
                if let Some(pkg) = scan_cellar_package(&paths.cellar, name)? {
                    import_cellar_package(installed, &pkg);
                    if !quiet {
                        println!(
                            "  {} Added {} {}",
                            style("✓").green(),
                            name,
                            style(version).dim()
                        );
                    }
                    applied += 1;
                }
            }
            DriftChange::FormulaRemoved { name, .. } => {
                installed.remove(name);
                if !quiet {
                    println!("  {} Removed {} from tracking", style("✓").green(), name);
                }
                applied += 1;
            }
            DriftChange::FormulaVersionChanged {
                name, new_version, ..
            } => {
                if let Some(pkg) = installed.get(name).cloned() {
                    if pkg.pinned {
                        if !quiet {
                            println!(
                                "  {} Skipped {} (pinned at {})",
                                style("!").yellow(),
                                name,
                                style(&pkg.version).dim()
                            );
                        }
                        continue;
                    }
                    let now = timestamp_now_iso();
                    installed.add_imported(
                        name,
                        new_version,
                        0,
                        pkg.requested,
                        "brew",
                        &now,
                        pkg.dependencies.clone(),
                    );
                    if !quiet {
                        println!(
                            "  {} Updated {} to {}",
                            style("✓").green(),
                            name,
                            style(new_version).cyan()
                        );
                    }
                    applied += 1;
                }
            }
            DriftChange::CaskAdded { token, version } => {
                let timestamp = timestamp_now_iso();
                let imported_cask = stout_cask::InstalledCask {
                    version: version.clone().unwrap_or_else(|| "unknown".to_string()),
                    installed_at: timestamp,
                    artifact_path: std::path::PathBuf::from(""),
                    auto_updates: false,
                    artifacts: Vec::new(),
                };
                cask_state.add(token, imported_cask);
                if !quiet {
                    println!("  {} Added cask {}", style("✓").green(), token);
                }
                applied += 1;
            }
            DriftChange::CaskRemoved { token } => {
                cask_state.remove(token);
                if !quiet {
                    println!(
                        "  {} Removed cask {} from tracking",
                        style("✓").green(),
                        token
                    );
                }
                applied += 1;
            }
        }
    }

    cask_state.save(&cask_state_path)?;

    Ok(applied)
}

/// Run sync in auto-apply mode (for use within stout update).
/// Prints informational output but doesn't prompt.
pub async fn run_auto_sync(paths: &Paths) -> Result<usize> {
    let mut installed = InstalledPackages::load(paths)?;
    let changes = detect_drift(&installed, paths)?;

    if changes.is_empty() {
        return Ok(0);
    }

    let applied = apply_changes(&mut installed, &changes, paths, false)?;

    if applied > 0 {
        installed.save(paths)?;
    }

    Ok(applied)
}

/// Detect and apply drift silently, returning a description of each change applied.
///
/// Unlike `run_auto_sync`, this does not print any output — the caller
/// controls formatting. Returns a vec of human-readable change descriptions.
pub async fn fix_drift(paths: &Paths) -> Result<Vec<String>> {
    let mut installed = InstalledPackages::load(paths)?;
    let changes = detect_drift(&installed, paths)?;

    if changes.is_empty() {
        return Ok(Vec::new());
    }

    let descriptions: Vec<String> = changes.iter().map(describe_change).collect();

    apply_changes(&mut installed, &changes, paths, true)?;

    installed.save(paths)?;

    Ok(descriptions)
}

/// Describe a drift change as a human-readable string.
fn describe_change(change: &DriftChange) -> String {
    match change {
        DriftChange::FormulaAdded { name, version } => {
            format!("added {} {}", name, version)
        }
        DriftChange::FormulaRemoved { name, .. } => {
            format!("removed {} from tracking", name)
        }
        DriftChange::FormulaVersionChanged {
            name, new_version, ..
        } => {
            format!("updated {} to {}", name, new_version)
        }
        DriftChange::CaskAdded { token, .. } => {
            format!("added cask {}", token)
        }
        DriftChange::CaskRemoved { token } => {
            format!("removed cask {}", token)
        }
    }
}
