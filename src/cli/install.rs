//! Install command

use anyhow::{bail, Context, Result};
use stout_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use stout_index::{Database, Formula, IndexSync};
use stout_install::{
    link_package, write_receipt, BottleInfo, BuildConfig, InstallReceipt,
    ParallelInstaller, RuntimeDependency, SourceBuilder,
};
use stout_resolve::{DependencyGraph, InstallPlan, InstallStep};
use stout_state::{Config, InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;
use std::sync::Arc;
use std::time::Instant;

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

    /// Build from source instead of using bottles
    #[arg(long, short = 's')]
    pub build_from_source: bool,

    /// Keep downloaded bottles after installation (don't cleanup)
    #[arg(long)]
    pub keep_bottles: bool,

    /// Number of parallel jobs for source builds
    #[arg(long, short = 'j')]
    pub jobs: Option<usize>,

    /// C compiler to use for source builds
    #[arg(long)]
    pub cc: Option<String>,

    /// C++ compiler to use for source builds
    #[arg(long)]
    pub cxx: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let start = Instant::now();

    if args.formulas.is_empty() {
        bail!("No formulas specified");
    }

    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    // Open database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'stout update' first.",
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

    // Fetch full formula data and categorize by installation method
    println!("\n{}...", style("Fetching formula data").cyan());
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let mut bottle_installs: Vec<(InstallStep, Formula)> = Vec::new();
    let mut source_installs: Vec<(InstallStep, Formula)> = Vec::new();
    let mut bottle_specs: Vec<BottleSpec> = Vec::new();

    for step in plan.new_packages() {
        let formula = sync
            .fetch_formula_cached(&step.name, None)
            .await
            .context(format!("Failed to fetch formula {}", step.name))?;

        // Check if we should build from source
        let use_source = args.build_from_source || formula.bottle_for_platform(&platform).is_none();

        if use_source {
            // Check if source is available
            if formula.urls.stable.is_none() {
                if !args.build_from_source {
                    bail!(
                        "No bottle for {} on {} and no source URL available",
                        step.name,
                        platform
                    );
                } else {
                    bail!("No source URL available for {}", step.name);
                }
            }
            source_installs.push((step.clone(), formula));
        } else {
            let bottle = formula.bottle_for_platform(&platform).unwrap();
            bottle_specs.push(BottleSpec {
                name: step.name.clone(),
                version: step.version.clone(),
                platform: platform.clone(),
                url: bottle.url.clone(),
                sha256: bottle.sha256.clone(),
            });
            bottle_installs.push((step.clone(), formula));
        }
    }

    // Download bottles in parallel
    let mut bottle_paths = Vec::new();
    if !bottle_specs.is_empty() {
        println!(
            "\n{} {} packages...",
            style("Downloading").cyan(),
            bottle_specs.len()
        );

        let cache = DownloadCache::new(&paths.stout_dir);
        let client = DownloadClient::new(cache, config.install.parallel_downloads as usize)?;
        let progress = Arc::new(ProgressReporter::new());

        bottle_paths = client
            .download_bottles(bottle_specs, Arc::clone(&progress))
            .await
            .context("Failed to download bottles")?;
    }

    // Install bottles (in parallel)
    if !bottle_installs.is_empty() {
        println!(
            "\n{} (parallel)...",
            style("Extracting and linking bottles").cyan()
        );

        // Prepare bottle info for parallel extraction
        let bottle_infos: Vec<BottleInfo> = bottle_installs
            .iter()
            .zip(bottle_paths.iter())
            .map(|((step, _), bottle_path)| BottleInfo {
                name: step.name.clone(),
                bottle_path: bottle_path.clone(),
            })
            .collect();

        // Run parallel installation
        let installer = ParallelInstaller::new();
        let install_results = installer
            .install_bottles(bottle_infos, &paths.cellar, &paths.prefix)
            .await
            .context("Parallel installation failed")?;

        // Write receipts and update state (must be sequential for state consistency)
        for ((step, formula), result) in bottle_installs.iter().zip(install_results.iter()) {
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
            write_receipt(&result.install_path, &receipt)?;

            installed.add(&step.name, &step.version, formula.revision, !step.is_dependency);
            println!(
                "  {} {} {} ({} files linked)",
                style("✓").green(),
                step.name,
                step.version,
                result.linked_files.len()
            );
        }
    }

    // Build from source
    if !source_installs.is_empty() {
        println!("\n{}...", style("Building from source").cyan());

        for (step, formula) in &source_installs {
            let source = formula.urls.stable.as_ref().unwrap();

            println!(
                "  {} {} {} (from source)",
                style("Building").yellow(),
                step.name,
                step.version
            );

            let build_config = BuildConfig {
                source_url: source.url.clone(),
                sha256: source.sha256.clone(),
                name: step.name.clone(),
                version: step.version.clone(),
                prefix: paths.prefix.clone(),
                cellar: paths.cellar.clone(),
                build_deps: formula.build_deps().to_vec(),
                jobs: args.jobs,
                cc: args.cc.clone(),
                cxx: args.cxx.clone(),
            };

            let work_dir = paths.stout_dir.join("build").join(&step.name);
            let builder = SourceBuilder::new(build_config, &work_dir);

            let result = builder.build().await.context(format!(
                "Failed to build {} from source",
                step.name
            ))?;

            // Link
            link_package(&result.install_path, &paths.prefix)?;

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

            let receipt = InstallReceipt::new_source(&formula.tap, !step.is_dependency, runtime_deps);
            write_receipt(&result.install_path, &receipt)?;

            installed.add(&step.name, &step.version, formula.revision, !step.is_dependency);

            // Cleanup build directory
            let _ = std::fs::remove_dir_all(&work_dir);

            println!("  {} {} {}", style("✓").green(), step.name, step.version);
        }
    }

    // Save state
    installed.save(&paths)?;

    // Cleanup downloaded bottles unless --keep-bottles is specified
    if !args.keep_bottles && !bottle_paths.is_empty() {
        let mut cleaned = 0u64;
        for path in &bottle_paths {
            if path.exists() {
                if let Ok(meta) = std::fs::metadata(path) {
                    cleaned += meta.len();
                }
                let _ = std::fs::remove_file(path);
            }
        }
        if cleaned > 0 {
            println!(
                "  {} Cleaned up {} of downloaded bottles",
                style("✓").dim(),
                format_bytes(cleaned)
            );
        }
    }

    let elapsed = start.elapsed();
    println!(
        "\n{} {} packages in {:.1}s",
        style("Installed").green().bold(),
        plan.total_packages(),
        elapsed.as_secs_f64()
    );

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
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
