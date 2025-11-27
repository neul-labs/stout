//! Deps command - show dependencies of a package

use anyhow::{bail, Context, Result};
use brewx_index::{Database, IndexSync};
use brewx_state::{Config, InstalledPackages, Paths};
use clap::{Args as ClapArgs, ValueEnum};
use console::style;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OutputFormat {
    #[default]
    List,
    Tree,
    Graph,
    Json,
}

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to show dependencies for
    pub formula: String,

    /// Show dependencies as a tree
    #[arg(long)]
    pub tree: bool,

    /// Output as DOT graph format
    #[arg(long)]
    pub graph: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Output format (list, tree, graph, json)
    #[arg(long, short = 'f', value_enum)]
    pub format: Option<OutputFormat>,

    /// Show all dependencies (including build and test)
    #[arg(long, short = 'a')]
    pub all: bool,

    /// Only show installed dependencies
    #[arg(long)]
    pub installed: bool,

    /// Include build dependencies
    #[arg(long)]
    pub include_build: bool,

    /// Include test dependencies
    #[arg(long)]
    pub include_test: bool,

    /// Include optional dependencies
    #[arg(long)]
    pub include_optional: bool,

    /// Show the dependency count
    #[arg(long, short = 'n')]
    pub count: bool,
}

/// JSON output structure for dependencies
#[derive(Debug, Serialize)]
pub struct DepsJson {
    pub formula: String,
    pub dependencies: Vec<DepInfo>,
    pub graph: Option<DepsGraph>,
}

#[derive(Debug, Serialize)]
pub struct DepInfo {
    pub name: String,
    pub installed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DepsGraph {
    pub nodes: Vec<String>,
    pub edges: Vec<(String, String)>,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    if !db.is_initialized()? {
        bail!("Index not initialized. Run 'brewx update' first.");
    }

    // Fetch full formula data
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.brewx_dir,
        config.security.to_security_policy(),
    )?;
    let formula = sync
        .fetch_formula_cached(&args.formula, None)
        .await
        .context(format!("Formula '{}' not found", args.formula))?;

    let installed = InstalledPackages::load(&paths)?;

    // Collect dependencies based on flags
    let mut deps: Vec<String> = Vec::new();

    // Runtime dependencies (always included)
    deps.extend(formula.runtime_deps().iter().cloned());

    // Build dependencies
    if args.all || args.include_build {
        deps.extend(formula.build_deps().iter().cloned());
    }

    // Test dependencies
    if args.all || args.include_test {
        deps.extend(formula.test_deps().iter().cloned());
    }

    // Optional dependencies
    if args.all || args.include_optional {
        deps.extend(formula.optional_deps().iter().cloned());
    }

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    deps.retain(|d| seen.insert(d.clone()));

    // Filter to installed only if requested
    if args.installed {
        deps.retain(|d| installed.is_installed(d));
    }

    if args.count {
        println!("{}", deps.len());
        return Ok(());
    }

    // Determine output format (flags take precedence over --format)
    let format = if args.graph {
        OutputFormat::Graph
    } else if args.json {
        OutputFormat::Json
    } else if args.tree {
        OutputFormat::Tree
    } else {
        args.format.unwrap_or(OutputFormat::List)
    };

    if deps.is_empty() && !matches!(format, OutputFormat::Json) {
        println!(
            "{} has no dependencies{}",
            style(&args.formula).cyan(),
            if args.installed { " installed" } else { "" }
        );
        return Ok(());
    }

    match format {
        OutputFormat::Tree => {
            println!("{}", style(&args.formula).cyan().bold());
            print_dep_tree(&sync, &installed, &deps, 1, &mut HashSet::new()).await?;
        }
        OutputFormat::Graph => {
            print_dot_graph(&args.formula, &sync, &deps, &installed).await?;
        }
        OutputFormat::Json => {
            print_json(&args.formula, &sync, &deps, &installed).await?;
        }
        OutputFormat::List => {
            for dep in &deps {
                let status = if installed.is_installed(dep) {
                    style("✓").green()
                } else {
                    style("○").dim()
                };
                println!("{} {}", status, dep);
            }
        }
    }

    Ok(())
}

/// Print dependencies in DOT graph format
async fn print_dot_graph(
    root: &str,
    sync: &IndexSync,
    deps: &[String],
    installed: &InstalledPackages,
) -> Result<()> {
    let mut nodes = HashSet::new();
    let mut edges = Vec::new();
    let mut visited = HashSet::new();

    nodes.insert(root.to_string());

    // Build graph recursively
    build_graph(root, deps, sync, &mut nodes, &mut edges, &mut visited).await?;

    // Output DOT format
    println!("digraph dependencies {{");
    println!("  rankdir=LR;");
    println!("  node [shape=box];");
    println!();

    // Style nodes based on installed status
    for node in &nodes {
        let color = if installed.is_installed(node) {
            "green"
        } else {
            "gray"
        };
        let style = if node == root { "bold" } else { "solid" };
        println!("  \"{}\" [color={}, style={}];", node, color, style);
    }
    println!();

    // Output edges
    for (from, to) in &edges {
        println!("  \"{}\" -> \"{}\";", from, to);
    }

    println!("}}");
    Ok(())
}

async fn build_graph(
    node: &str,
    deps: &[String],
    sync: &IndexSync,
    nodes: &mut HashSet<String>,
    edges: &mut Vec<(String, String)>,
    visited: &mut HashSet<String>,
) -> Result<()> {
    if visited.contains(node) {
        return Ok(());
    }
    visited.insert(node.to_string());

    for dep in deps {
        nodes.insert(dep.clone());
        edges.push((node.to_string(), dep.clone()));

        // Recursively add subdependencies
        if let Ok(formula) = sync.fetch_formula_cached(dep, None).await {
            let subdeps: Vec<String> = formula.runtime_deps().iter().cloned().collect();
            if !subdeps.is_empty() {
                Box::pin(build_graph(dep, &subdeps, sync, nodes, edges, visited)).await?;
            }
        }
    }
    Ok(())
}

/// Print dependencies as JSON
async fn print_json(
    root: &str,
    sync: &IndexSync,
    deps: &[String],
    installed: &InstalledPackages,
) -> Result<()> {
    let mut dep_infos = Vec::new();
    let mut nodes = HashSet::new();
    let mut edges = Vec::new();
    let mut visited = HashSet::new();

    nodes.insert(root.to_string());

    // Build info for each dependency
    for dep in deps {
        let is_installed = installed.is_installed(dep);
        let version = installed.get(dep).map(|p| p.version.clone());

        let subdeps = if let Ok(formula) = sync.fetch_formula_cached(dep, None).await {
            formula.runtime_deps().to_vec()
        } else {
            Vec::new()
        };

        dep_infos.push(DepInfo {
            name: dep.clone(),
            installed: is_installed,
            version,
            dependencies: subdeps,
        });
    }

    // Build graph for JSON output
    build_graph(root, deps, sync, &mut nodes, &mut edges, &mut visited).await?;

    let output = DepsJson {
        formula: root.to_string(),
        dependencies: dep_infos,
        graph: Some(DepsGraph {
            nodes: nodes.into_iter().collect(),
            edges,
        }),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

async fn print_dep_tree(
    sync: &IndexSync,
    installed: &InstalledPackages,
    deps: &[String],
    depth: usize,
    visited: &mut HashSet<String>,
) -> Result<()> {
    for (i, dep) in deps.iter().enumerate() {
        let is_last = i == deps.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };
        let indent = "│   ".repeat(depth.saturating_sub(1));

        let status = if installed.is_installed(dep) {
            style("✓").green()
        } else {
            style("○").dim()
        };

        println!("{}{}{} {}", indent, prefix, status, dep);

        // Recursively show dependencies (avoid cycles)
        if !visited.contains(dep) {
            visited.insert(dep.clone());

            if let Ok(formula) = sync.fetch_formula_cached(dep, None).await {
                let subdeps: Vec<String> = formula.runtime_deps().iter().cloned().collect();
                if !subdeps.is_empty() {
                    let child_indent = if is_last { "    " } else { "│   " };
                    let new_indent = format!("{}{}", indent, child_indent);
                    print_dep_tree_with_indent(sync, installed, &subdeps, &new_indent, visited)
                        .await?;
                }
            }
        }
    }
    Ok(())
}

async fn print_dep_tree_with_indent(
    sync: &IndexSync,
    installed: &InstalledPackages,
    deps: &[String],
    base_indent: &str,
    visited: &mut HashSet<String>,
) -> Result<()> {
    for (i, dep) in deps.iter().enumerate() {
        let is_last = i == deps.len() - 1;
        let prefix = if is_last { "└── " } else { "├── " };

        let status = if installed.is_installed(dep) {
            style("✓").green()
        } else {
            style("○").dim()
        };

        println!("{}{}{} {}", base_indent, prefix, status, dep);

        if !visited.contains(dep) {
            visited.insert(dep.clone());

            if let Ok(formula) = sync.fetch_formula_cached(dep, None).await {
                let subdeps: Vec<String> = formula.runtime_deps().iter().cloned().collect();
                if !subdeps.is_empty() {
                    let child_indent = if is_last { "    " } else { "│   " };
                    let new_indent = format!("{}{}", base_indent, child_indent);
                    Box::pin(print_dep_tree_with_indent(
                        sync, installed, &subdeps, &new_indent, visited,
                    ))
                    .await?;
                }
            }
        }
    }
    Ok(())
}
