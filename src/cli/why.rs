//! Why command - show why a package is installed (reverse dependency chain)

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use console::style;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use stout_index::{Database, DependencyType};
use stout_state::{Config, InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to find the installation reason for
    pub formula: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Show all dependency paths (not just shortest)
    #[arg(long, short = 'a')]
    pub all: bool,
}

/// JSON output structure
#[derive(Debug, Serialize)]
pub struct WhyJson {
    pub formula: String,
    pub installed: bool,
    pub requested: bool,
    pub paths: Vec<Vec<String>>,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let _config = Config::load(&paths)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        bail!("Index not initialized. Run 'stout update' first.");
    }

    let installed = InstalledPackages::load(&paths)?;

    // Check if package is installed
    if !installed.is_installed(&args.formula) {
        if args.json {
            let output = WhyJson {
                formula: args.formula.clone(),
                installed: false,
                requested: false,
                paths: vec![],
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{} is not installed", style(&args.formula).cyan());
        }
        return Ok(());
    }

    // Check if it was explicitly requested
    let pkg = installed.get(&args.formula).with_context(|| {
        format!(
            "package '{}' is installed but not found in state",
            args.formula
        )
    })?;
    if pkg.requested {
        if args.json {
            let output = WhyJson {
                formula: args.formula.clone(),
                installed: true,
                requested: true,
                paths: vec![vec![args.formula.clone()]],
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!(
                "{} was explicitly installed (requested)",
                style(&args.formula).cyan().bold()
            );
        }
        return Ok(());
    }

    // Find dependency paths from requested packages to this formula
    let dep_paths = find_dependency_paths(&args.formula, &installed, Some(&db), args.all);

    if args.json {
        let output = WhyJson {
            formula: args.formula.clone(),
            installed: true,
            requested: false,
            paths: dep_paths.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if dep_paths.is_empty() {
        println!(
            "{} is installed but no dependency path found (orphan)",
            style(&args.formula).cyan()
        );
        println!(
            "  {} Consider running 'stout autoremove' to clean up",
            style("Hint:").yellow()
        );
    } else {
        println!(
            "{} is installed because:",
            style(&args.formula).cyan().bold()
        );
        println!();

        for (i, path) in dep_paths.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print_dependency_path(path);
        }
    }

    Ok(())
}

/// Find paths from requested packages to the target formula
fn find_dependency_paths(
    target: &str,
    installed: &InstalledPackages,
    db: Option<&Database>,
    find_all: bool,
) -> Vec<Vec<String>> {
    let mut paths = Vec::new();

    // Build reverse dependency map: package -> packages that depend on it
    let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
    for (name, pkg) in installed.packages.iter() {
        for dep in &pkg.dependencies {
            reverse_deps
                .entry(dep.clone())
                .or_default()
                .push(name.clone());
        }
    }

    // Supplement with database-sourced reverse dependencies
    if let Some(database) = db {
        for name in installed.names() {
            if let Ok(dependents) =
                database.get_dependents(name, DependencyType::default_dependent_types())
            {
                for dep in dependents {
                    if installed.is_installed(&dep.formula) {
                        reverse_deps
                            .entry(name.to_string())
                            .or_default()
                            .push(dep.formula);
                    }
                }
            }
        }

        // Deduplicate reverse deps entries
        for dependents in reverse_deps.values_mut() {
            dependents.sort();
            dependents.dedup();
        }
    }

    // BFS from target to find paths to requested packages
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    let mut visited: HashSet<String> = HashSet::new();

    queue.push_back(vec![target.to_string()]);

    while let Some(current_path) = queue.pop_front() {
        let current = current_path
            .last()
            .expect("current_path should always have at least one element");

        if visited.contains(current) && !find_all {
            continue;
        }
        visited.insert(current.clone());

        // Check if this is a requested package
        if let Some(pkg) = installed.get(current) {
            if pkg.requested {
                paths.push(current_path.clone());
                if !find_all {
                    // Found shortest path, but continue to find other root causes
                    continue;
                }
            }
        }

        // Add packages that depend on current
        if let Some(dependents) = reverse_deps.get(current) {
            for dependent in dependents {
                if !current_path.contains(dependent) {
                    // Avoid cycles
                    let mut new_path = current_path.clone();
                    new_path.push(dependent.clone());
                    queue.push_back(new_path);
                }
            }
        }
    }

    // Reverse paths so they go from root -> target
    for path in &mut paths {
        path.reverse();
    }

    // Sort by length (shortest first)
    paths.sort_by_key(|p| p.len());

    // Deduplicate
    paths.dedup();

    paths
}

/// Print a dependency path with nice formatting
fn print_dependency_path(path: &[String]) {
    for (i, pkg) in path.iter().enumerate() {
        let indent = "  ".repeat(i);
        let prefix = if i == 0 {
            style("●").green().to_string()
        } else {
            style("└─▶").dim().to_string()
        };

        let pkg_style = if i == 0 {
            style(pkg).green().bold()
        } else if i == path.len() - 1 {
            style(pkg).cyan().bold()
        } else {
            style(pkg).white()
        };

        let suffix = if i == 0 {
            style(" (requested)").dim().to_string()
        } else if i == path.len() - 1 {
            String::new()
        } else {
            style(" depends on").dim().to_string()
        };

        println!("{}{} {}{}", indent, prefix, pkg_style, suffix);
    }
}
