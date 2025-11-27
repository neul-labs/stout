//! Upgrade command

use anyhow::{Context, Result};
use brewx_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use brewx_index::{Database, Formula, IndexSync};
use brewx_install::{
    extract_bottle, link_package, remove_package, unlink_package, write_receipt, InstallReceipt,
    RuntimeDependency,
};
use brewx_state::{Config, InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;
use std::sync::Arc;
use std::time::Instant;

#[derive(ClapArgs)]
pub struct Args {
    /// Specific formulas to upgrade (all if none specified)
    pub formulas: Vec<String>,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

struct UpgradeCandidate {
    name: String,
    old_version: String,
    new_version: String,
    explicitly_requested: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let start = Instant::now();
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    let mut installed = InstalledPackages::load(&paths)?;

    println!("\n{}...", style("Checking for updates").cyan());

    // Find upgradable packages
    let mut upgradable: Vec<UpgradeCandidate> = Vec::new();

    let packages_to_check: Vec<_> = if args.formulas.is_empty() {
        installed.names().cloned().collect()
    } else {
        args.formulas.clone()
    };

    for name in packages_to_check {
        let pkg = match installed.get(&name) {
            Some(pkg) => pkg,
            None => continue,
        };

        let info = match db.get_formula(&name)? {
            Some(info) => info,
            None => continue,
        };

        if info.version != pkg.version {
            upgradable.push(UpgradeCandidate {
                name: name.clone(),
                old_version: pkg.version.clone(),
                new_version: info.version.clone(),
                explicitly_requested: pkg.requested,
            });
        }
    }

    if upgradable.is_empty() {
        println!("\n{}", style("All packages are up to date.").green());
        return Ok(());
    }

    // Show upgradable
    println!("\n{} packages can be upgraded:\n", upgradable.len());

    let max_name = upgradable.iter().map(|u| u.name.len()).max().unwrap_or(0);
    let max_old = upgradable
        .iter()
        .map(|u| u.old_version.len())
        .max()
        .unwrap_or(0);

    println!(
        "  {:<name_w$}  {:<old_w$}     {}",
        style("Package").dim(),
        style("Current").dim(),
        style("Latest").dim(),
        name_w = max_name,
        old_w = max_old,
    );
    println!(
        "  {:<name_w$}",
        style("─".repeat(max_name + max_old + 15)).dim(),
        name_w = 0,
    );

    for candidate in &upgradable {
        println!(
            "  {:<name_w$}  {:<old_w$}  →  {}",
            style(&candidate.name).green(),
            &candidate.old_version,
            style(&candidate.new_version).cyan(),
            name_w = max_name,
            old_w = max_old,
        );
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Detect platform
    let platform = detect_platform();

    // Fetch formula data and prepare bottle specs
    println!("\n{}...", style("Fetching formula data").cyan());
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.brewx_dir,
        config.security.to_security_policy(),
    )?;

    let mut formulas_to_upgrade: Vec<(UpgradeCandidate, Formula)> = Vec::new();
    let mut bottle_specs: Vec<BottleSpec> = Vec::new();

    for candidate in upgradable {
        let formula = sync
            .fetch_formula_cached(&candidate.name, None)
            .await
            .context(format!("Failed to fetch formula {}", candidate.name))?;

        let bottle = formula
            .bottle_for_platform(&platform)
            .context(format!(
                "No bottle for {} on {}",
                candidate.name, platform
            ))?;

        bottle_specs.push(BottleSpec {
            name: candidate.name.clone(),
            version: candidate.new_version.clone(),
            platform: platform.clone(),
            url: bottle.url.clone(),
            sha256: bottle.sha256.clone(),
        });

        formulas_to_upgrade.push((candidate, formula));
    }

    // Download all bottles in parallel
    println!(
        "\n{} {} packages...",
        style("Downloading").cyan(),
        bottle_specs.len()
    );

    let cache = DownloadCache::new(&paths.brewx_dir);
    let client = DownloadClient::new(cache, config.install.parallel_downloads as usize)?;
    let progress = Arc::new(ProgressReporter::new());

    let bottle_paths = client
        .download_bottles(bottle_specs, Arc::clone(&progress))
        .await
        .context("Failed to download bottles")?;

    // Upgrade all packages
    println!("\n{}...", style("Upgrading").cyan());

    for ((candidate, formula), bottle_path) in
        formulas_to_upgrade.iter().zip(bottle_paths.iter())
    {
        // First, unlink old version
        let old_install_path = paths.package_path(&candidate.name, &candidate.old_version);
        if old_install_path.exists() {
            unlink_package(&old_install_path, &paths.prefix)?;
            remove_package(&paths.cellar, &candidate.name, &candidate.old_version)?;
        }

        // Extract new version
        let install_path = extract_bottle(bottle_path, &paths.cellar)?;

        // Link new version
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

        let receipt =
            InstallReceipt::new_bottle(&formula.tap, candidate.explicitly_requested, runtime_deps);
        write_receipt(&install_path, &receipt)?;

        // Update state - remove old, add new
        installed.remove(&candidate.name);
        installed.add(
            &candidate.name,
            &candidate.new_version,
            formula.revision,
            candidate.explicitly_requested,
        );

        println!(
            "  {} {} {} → {}",
            style("✓").green(),
            candidate.name,
            style(&candidate.old_version).dim(),
            style(&candidate.new_version).cyan()
        );
    }

    // Save state
    installed.save(&paths)?;

    let elapsed = start.elapsed();
    println!(
        "\n{} {} packages in {:.1}s",
        style("Upgraded").green().bold(),
        formulas_to_upgrade.len(),
        elapsed.as_secs_f64()
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
        format!("{}_sonoma", arch)
    } else {
        format!("{}_linux", arch)
    }
}
