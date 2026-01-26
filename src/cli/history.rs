//! History command - show package version history

use anyhow::{bail, Result};
use stout_state::{HistoryAction, PackageHistory, Paths};
use clap::Args as ClapArgs;
use console::style;
use serde::Serialize;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to show history for (omit for all packages)
    pub formula: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Show only the last N entries
    #[arg(long, short = 'n')]
    pub limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct HistoryOutput {
    packages: Vec<PackageHistoryOutput>,
}

#[derive(Debug, Serialize)]
struct PackageHistoryOutput {
    name: String,
    entries: Vec<EntryOutput>,
}

#[derive(Debug, Serialize)]
struct EntryOutput {
    version: String,
    revision: u32,
    action: String,
    timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from_version: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let history = PackageHistory::load(&paths)?;

    if let Some(formula) = &args.formula {
        // Show history for specific formula
        let entries = history.get(formula);

        if entries.as_ref().map_or(true, |e| e.is_empty()) {
            if args.json {
                println!("{{\"packages\": []}}");
            } else {
                println!("No history found for {}", style(formula).cyan());
            }
            return Ok(());
        }

        let entries = entries.expect("checked that entries is Some above");
        let entries: Vec<_> = if let Some(limit) = args.limit {
            entries.iter().rev().take(limit).collect()
        } else {
            entries.iter().rev().collect()
        };

        if args.json {
            let output = HistoryOutput {
                packages: vec![PackageHistoryOutput {
                    name: formula.clone(),
                    entries: entries
                        .iter()
                        .map(|e| EntryOutput {
                            version: e.version.clone(),
                            revision: e.revision,
                            action: e.action.as_str().to_string(),
                            timestamp: e.timestamp.clone(),
                            from_version: e.from_version.clone(),
                        })
                        .collect(),
                }],
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "{} History for {}",
                style("==>").blue().bold(),
                style(formula).cyan().bold()
            );
            println!();

            for entry in entries {
                print_entry(formula, entry);
            }
        }
    } else {
        // Show history for all packages
        if history.packages.is_empty() {
            if args.json {
                println!("{{\"packages\": []}}");
            } else {
                println!("No package history found");
            }
            return Ok(());
        }

        if args.json {
            let mut packages = Vec::new();
            for (name, entries) in &history.packages {
                let entries: Vec<_> = if let Some(limit) = args.limit {
                    entries.iter().rev().take(limit).collect()
                } else {
                    entries.iter().rev().collect()
                };

                packages.push(PackageHistoryOutput {
                    name: name.clone(),
                    entries: entries
                        .iter()
                        .map(|e| EntryOutput {
                            version: e.version.clone(),
                            revision: e.revision,
                            action: e.action.as_str().to_string(),
                            timestamp: e.timestamp.clone(),
                            from_version: e.from_version.clone(),
                        })
                        .collect(),
                });
            }
            packages.sort_by(|a, b| a.name.cmp(&b.name));

            let output = HistoryOutput { packages };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "{} Package History ({} packages)",
                style("==>").blue().bold(),
                history.packages.len()
            );
            println!();

            let mut names: Vec<_> = history.packages.keys().collect();
            names.sort();

            for name in names {
                let entries = history.packages.get(name)
                    .ok_or_else(|| anyhow::anyhow!("package '{}' in history but not found", name))?;
                let entries: Vec<_> = if let Some(limit) = args.limit {
                    entries.iter().rev().take(limit).collect()
                } else {
                    entries.iter().rev().collect()
                };

                println!("{}", style(name).cyan().bold());
                for entry in entries {
                    print_entry(name, entry);
                }
                println!();
            }
        }
    }

    Ok(())
}

fn print_entry(name: &str, entry: &stout_state::HistoryEntry) {
    let action_style = match entry.action {
        HistoryAction::Install => style("install").green(),
        HistoryAction::Upgrade => style("upgrade").blue(),
        HistoryAction::Downgrade => style("downgrade").yellow(),
        HistoryAction::Reinstall => style("reinstall").cyan(),
        HistoryAction::Uninstall => style("uninstall").red(),
    };

    let version_str = if entry.revision > 0 {
        format!("{}_{}", entry.version, entry.revision)
    } else {
        entry.version.clone()
    };

    let from_str = if let Some(from) = &entry.from_version {
        let from_rev = entry.from_revision.unwrap_or(0);
        if from_rev > 0 {
            format!(" (from {}_{}", from, from_rev)
        } else {
            format!(" (from {})", from)
        }
    } else {
        String::new()
    };

    println!(
        "  {} {} {} {}{}",
        style(&entry.timestamp).dim(),
        action_style,
        style(&version_str).white().bold(),
        style(&from_str).dim(),
        ""
    );
}
