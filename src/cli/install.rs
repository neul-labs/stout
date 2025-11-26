//! Install command

use anyhow::{bail, Context, Result};
use brewx_fetch::{DownloadCache, DownloadClient, ProgressReporter};
use brewx_index::{Database, IndexSync};
use brewx_install::{extract_bottle, link_package, write_receipt, InstallReceipt, RuntimeDependency};
use brewx_resolve::{DependencyGraph, InstallPlan};
use brewx_state::{Config, InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to install
    pub formulas: Vec<String>,

    /// Don't install dependencies
    #[arg(long)]
    pub ignore_dependencies: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No formulas specified");
    }

    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    // Open database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'brewx update' first.",
            style("error:").red().bold()
        );
        std::process::exit(1);
    }

    // Verify all formulas exist
    for name in &args.formulas {
        if db.get_formula(name)?.is_none() {
            let suggestions = db.find_similar(name, 3)?;
            eprintln!(
                "\n{} formula '{}' not found",
                style("error:").red().bold(),
                name
            );
            if !suggestions.is_empty() {
                eprintln!("\n{}:", style("Did you mean?").yellow());
                for s in suggestions {
                    eprintln!("  {} {}", style("•").dim(), s);
                }
            }
            std::process::exit(1);
        }
    }

    // Load installed packages
    let mut installed = InstalledPackages::load(&paths)?;

    // Build dependency graph
    println!("\n{}...", style("Resolving dependencies").cyan());

    let formula_refs: Vec<&str> = args.formulas.iter().map(|s| s.as_str()).collect();
    let graph = DependencyGraph::build_from_db(&db, &formula_refs, false)?;

    // Create install plan
    let plan = InstallPlan::from_graph(
        &graph,
        &formula_refs,
        |name| db.get_formula(name).ok().flatten(),
        |name| installed.is_installed(name),
    )?;

    // Show plan
    for step in plan.new_packages() {
        if step.is_dependency {
            println!(
                "  {} {} {} {}",
                style("+").green(),
                step.name,
                style(&step.version).dim(),
                style("(dependency)").dim()
            );
        } else {
            println!(
                "  {} {} {}",
                style("✓").green(),
                step.name,
                style(&step.version).dim()
            );
        }
    }

    if !plan.already_installed.is_empty() {
        for name in &plan.already_installed {
            println!(
                "  {} {} {}",
                style("•").dim(),
                name,
                style("(already installed)").dim()
            );
        }
    }

    if plan.is_empty() {
        println!("\n{}", style("Nothing to install.").dim());
        return Ok(());
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Detect platform
    let platform = detect_platform();
    println!("\n{}...", style("Downloading").cyan());

    // Fetch full formula data and prepare downloads
    let sync = IndexSync::new(Some(&config.index.base_url), &paths.brewx_dir)?;
    let cache = DownloadCache::new(&paths.brewx_dir);
    let client = DownloadClient::new(cache)?;
    let progress = ProgressReporter::new();

    for step in plan.new_packages() {
        let formula = sync
            .fetch_formula_cached(&step.name, None)
            .await
            .context(format!("Failed to fetch formula {}", step.name))?;

        let bottle = formula
            .bottle_for_platform(&platform)
            .context(format!("No bottle for {} on {}", step.name, platform))?;

        // Download
        let bottle_path = client
            .download_bottle(
                &step.name,
                &step.version,
                &platform,
                &bottle.url,
                &bottle.sha256,
                Some(&progress),
            )
            .await?;

        // Extract
        println!("  {} {}", style("Installing").cyan(), step.name);
        let install_path = extract_bottle(&bottle_path, &paths.cellar)?;

        // Link
        link_package(&install_path, &paths.prefix)?;

        // Write receipt
        let runtime_deps: Vec<RuntimeDependency> = formula
            .runtime_deps()
            .iter()
            .filter_map(|dep| {
                db.get_formula(dep).ok().flatten().map(|info| RuntimeDependency {
                    full_name: dep.clone(),
                    version: info.version,
                    revision: Some(info.revision),
                })
            })
            .collect();

        let receipt = InstallReceipt::new_bottle(&formula.tap, !step.is_dependency, runtime_deps);
        write_receipt(&install_path, &receipt)?;

        // Track in state
        installed.add(&step.name, &step.version, formula.revision, !step.is_dependency);

        println!("  {} {} {}", style("✓").green(), step.name, step.version);
    }

    // Save state
    installed.save(&paths)?;

    println!(
        "\n{} {} packages in {:?}",
        style("Installed").green().bold(),
        plan.total_packages(),
        std::time::Duration::from_secs(0) // TODO: actual timing
    );

    Ok(())
}

fn detect_platform() -> String {
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };

    if cfg!(target_os = "macos") {
        // Try to detect macOS version
        // Default to sonoma for now
        format!("{}_sonoma", arch)
    } else {
        format!("{}_linux", arch)
    }
}
