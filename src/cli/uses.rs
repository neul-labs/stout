//! Uses command - show packages that depend on a given package

use std::collections::{HashSet, VecDeque};

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_index::{Database, DependencyType, Dependent};
use stout_state::{InstalledPackages, Paths};

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

fn build_dep_types(args: &Args) -> Vec<DependencyType> {
    let mut dep_types = vec![DependencyType::Runtime, DependencyType::Recommended];
    if args.include_build {
        dep_types.push(DependencyType::Build);
    }
    if args.include_test {
        dep_types.push(DependencyType::Test);
    }
    if args.include_optional {
        dep_types.push(DependencyType::Optional);
    }
    dep_types
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
    let dep_types = build_dep_types(&args);

    let dependents = if args.recursive {
        find_recursive_dependents(&args.formula, &db, &installed, &dep_types, args.installed)?
    } else {
        db.get_dependents(&args.formula, &dep_types)?
    };

    // Filter by installed status
    let results: Vec<Dependent> = if args.installed {
        dependents
            .into_iter()
            .filter(|d| installed.is_installed(&d.formula))
            .collect()
    } else {
        dependents
    };

    if results.is_empty() {
        println!(
            "No {} packages depend on {}",
            if args.installed { "installed" } else { "" },
            style(&args.formula).cyan()
        );
        return Ok(());
    }

    // Deduplicate by formula name (keep first occurrence which has the primary dep_type)
    let mut seen = HashSet::new();
    let results: Vec<Dependent> = results
        .into_iter()
        .filter(|d| seen.insert(d.formula.clone()))
        .collect();

    println!(
        "{} {} {} package{} that {} {}:",
        style("==>").blue().bold(),
        results.len(),
        if args.installed { "installed" } else { "" },
        if results.len() == 1 { "" } else { "s" },
        if results.len() == 1 { "uses" } else { "use" },
        style(&args.formula).cyan()
    );

    for dep in &results {
        let version = installed
            .get(&dep.formula)
            .map(|p| p.version.as_str())
            .unwrap_or_default();
        let marker = if installed.is_installed(&dep.formula) {
            style("✓").green()
        } else {
            style("○").dim()
        };
        println!(
            "  {} {} {} {}",
            marker,
            dep.formula,
            style(version).dim(),
            style(format!("({})", dep.dep_type.as_str())).dim()
        );
    }

    Ok(())
}

fn find_recursive_dependents(
    formula: &str,
    db: &Database,
    installed: &InstalledPackages,
    dep_types: &[DependencyType],
    only_installed: bool,
) -> Result<Vec<Dependent>> {
    let mut visited = HashSet::new();
    visited.insert(formula.to_string());
    let mut queue = VecDeque::new();
    queue.push_back(formula.to_string());
    let mut all_dependents = Vec::new();

    while let Some(current) = queue.pop_front() {
        let dependents = db.get_dependents(&current, dep_types)?;
        for dep in dependents {
            if visited.insert(dep.formula.clone())
                && (!only_installed || installed.is_installed(&dep.formula))
            {
                all_dependents.push(Dependent {
                    formula: dep.formula.clone(),
                    dep_type: dep.dep_type,
                });
                queue.push_back(dep.formula.clone());
            }
        }
    }

    Ok(all_dependents)
}
