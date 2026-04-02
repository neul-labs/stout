//! Upgrade command

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Instant;
use stout_audit::compare_versions;
use stout_cask::InstalledCasks;
use stout_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use stout_index::{Database, Formula, IndexSync};
use stout_install::{
    extract_bottle, link_package, relocate_bottle, remove_package, scan_cellar_package,
    unlink_package, write_receipt, InstallReceipt, RuntimeDependency,
};
use stout_state::{Config, InstalledPackages, Paths};
use tracing::warn;

// 5-minute timeout per cask install (covers large app bundles like Handbrake/Raycast)
const CASK_INSTALL_TIMEOUT_SECS: u64 = 300;

#[derive(ClapArgs)]
pub struct Args {
    /// Specific formulas to upgrade (all if none specified)
    pub formulas: Vec<String>,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,

    /// Check HEAD packages for updates (does not upgrade, use reinstall)
    #[arg(long = "fetch-HEAD")]
    pub fetch_head: bool,

    /// Only upgrade casks
    #[arg(long)]
    pub cask: bool,

    /// Only upgrade formulas
    #[arg(long, conflicts_with = "cask")]
    pub formula: bool,
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

    // Auto-update index if needed (unless --no-update flag is set)
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    // Check if index is outdated and auto-update
    if let Ok(Some(manifest)) = sync.check_update(&db).await {
        let current_version = db
            .version()
            .ok()
            .flatten()
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "{} Index is outdated ({} → {}). Updating...",
            style("!").yellow(),
            current_version,
            manifest.version
        );
        sync.sync_index(paths.index_db()).await?;
        println!("  {} Index updated", style("✓").green());
    }

    // Reopen database after potential update
    let db = Database::open(paths.index_db()).context("Failed to open index after update")?;

    let mut installed = InstalledPackages::load(&paths)?;

    println!("\n{}...", style("Checking for updates").cyan());

    let mut upgradable: Vec<UpgradeCandidate> = Vec::new();
    let mut pinned_skipped: Vec<String> = Vec::new();
    let mut head_updates: Vec<(String, String, String)> = Vec::new();

    // Find upgradable formulas (skip if --cask)
    if !args.cask {
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

            // Skip HEAD formulas - they are not upgraded to stable versions
            if pkg.is_head_install() {
                continue;
            }

            // Skip pinned formulas unless explicitly requested by name
            if pkg.pinned && args.formulas.is_empty() {
                // Check if there's actually an update available before reporting
                if let Some(info) = db.get_formula(&name)? {
                    let installed_ver = effective_version(&pkg.version, pkg.revision);
                    let index_ver = effective_version(&info.version, info.revision);
                    if compare_versions(&installed_ver, &index_ver) == Ordering::Less {
                        pinned_skipped.push(name.clone());
                    }
                }
                continue;
            }

            let info = match db.get_formula(&name)? {
                Some(info) => info,
                None => continue,
            };

            // Only mark as upgradable if installed version is strictly less than current.
            // Incorporate revision into the comparison so rebuilds (e.g. 25.9.0_1) are
            // detected even when the base version string is unchanged.
            let installed_ver = effective_version(&pkg.version, pkg.revision);
            let index_ver = effective_version(&info.version, info.revision);
            if compare_versions(&installed_ver, &index_ver) == Ordering::Less {
                upgradable.push(UpgradeCandidate {
                    name: name.clone(),
                    old_version: installed_ver,
                    new_version: index_ver,
                    explicitly_requested: pkg.requested,
                });
            }
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
                        head_updates.push((
                            name.clone(),
                            pkg.short_sha().unwrap_or("?").to_string(),
                            short_remote,
                        ));
                    }
                }
            }
        }
    } // end if !args.cask

    // Check for cask upgrades (skip if --formula)
    let mut cask_upgrades: Vec<(String, String, String)> = Vec::new(); // (token, old, new)
    let cask_state_path = paths.stout_dir.join("casks.json");
    if !args.formula {
        if let Ok(installed_casks) = InstalledCasks::load(&cask_state_path) {
            for (token, cask) in installed_casks.iter() {
                // Skip if version is unknown
                if cask.version == "unknown" {
                    continue;
                }

                // Check if there's a newer version in the index
                if let Some(info) = db.get_cask(token)? {
                    if compare_versions(&cask.version, &info.version) == Ordering::Less {
                        cask_upgrades.push((token.clone(), cask.version.clone(), info.version));
                    }
                }
            }
        }
    } // end if !args.formula

    if !pinned_skipped.is_empty() {
        println!(
            "\n{} {} pinned {} skipped (use 'stout unpin' to allow upgrades)",
            style("!").yellow(),
            pinned_skipped.len(),
            if pinned_skipped.len() == 1 {
                "package"
            } else {
                "packages"
            }
        );
    }

    if upgradable.is_empty() && head_updates.is_empty() && cask_upgrades.is_empty() {
        println!("\n{}", style("All packages are up to date.").green());
        return Ok(());
    }

    // Show upgradable formulas
    if !upgradable.is_empty() {
        println!(
            "\n{} {} can be upgraded:\n",
            upgradable.len(),
            if upgradable.len() == 1 {
                "formula"
            } else {
                "formulas"
            }
        );

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
    }

    // Show cask upgrades if any
    if !cask_upgrades.is_empty() {
        println!(
            "\n{} {} can be upgraded:\n",
            cask_upgrades.len(),
            if cask_upgrades.len() == 1 {
                "cask"
            } else {
                "casks"
            }
        );

        let max_token = cask_upgrades
            .iter()
            .map(|(t, _, _)| t.len())
            .max()
            .unwrap_or(0);
        let max_old = cask_upgrades
            .iter()
            .map(|(_, o, _)| o.len())
            .max()
            .unwrap_or(0);

        println!(
            "  {:<token_w$}  {:<old_w$}     {}",
            style("Cask").dim(),
            style("Current").dim(),
            style("Latest").dim(),
            token_w = max_token,
            old_w = max_old,
        );
        println!(
            "  {:<token_w$}",
            style("─".repeat(max_token + max_old + 15)).dim(),
            token_w = 0,
        );

        for (token, old, new) in &cask_upgrades {
            println!(
                "  {:<token_w$}  {:<old_w$}  →  {}",
                style(token).magenta(),
                old,
                style(new).cyan(),
                token_w = max_token,
                old_w = max_old,
            );
        }
    }

    // Show HEAD updates if any
    if !head_updates.is_empty() {
        println!("\n{} HEAD packages have updates:", head_updates.len());

        for (name, old_sha, new_sha) in &head_updates {
            println!(
                "  {} {} {} → {}",
                style("~").yellow(),
                style(name).green(),
                style(old_sha).dim(),
                style(new_sha).cyan()
            );
        }

        println!(
            "\n  {}",
            style("Use 'stout reinstall <package>' to update HEAD packages").dim()
        );
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Targeted Cellar pre-check: detect packages already upgraded externally (e.g. by brew)
    let mut already_upgraded: Vec<String> = Vec::new();
    let remaining_upgradable: Vec<UpgradeCandidate> = upgradable
        .into_iter()
        .filter(|candidate| {
            if let Ok(Some(cellar_pkg)) = scan_cellar_package(&paths.cellar, &candidate.name) {
                if cellar_pkg.version == candidate.new_version {
                    // Already at target version in Cellar — just update state
                    // Preserve installed_by since stout didn't perform this upgrade
                    if let Some(existing) = installed.get(&candidate.name) {
                        let existing = existing.clone();
                        installed.add_imported(
                            &candidate.name,
                            &candidate.new_version,
                            0,
                            candidate.explicitly_requested,
                            &existing.installed_by,
                            &existing.installed_at,
                            existing.dependencies.clone(),
                        );
                    } else {
                        installed.add(
                            &candidate.name,
                            &candidate.new_version,
                            0,
                            candidate.explicitly_requested,
                        );
                    }
                    println!(
                        "  {} {} {} → {} {}",
                        style("✓").green(),
                        candidate.name,
                        style(&candidate.old_version).dim(),
                        style(&candidate.new_version).cyan(),
                        style("(already upgraded externally)").dim()
                    );
                    already_upgraded.push(candidate.name.clone());
                    return false;
                }
            }
            true
        })
        .collect();

    let upgradable = remaining_upgradable;

    if upgradable.is_empty() && !already_upgraded.is_empty() {
        installed.save(&paths)?;
        println!(
            "\n{} {} packages (all already upgraded externally)",
            style("Updated state for").green().bold(),
            already_upgraded.len()
        );
        return Ok(());
    }

    // Detect platform
    let platform = super::detect_platform();

    // Fetch formula data and prepare bottle specs
    println!("\n{}...", style("Fetching package data").cyan());
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let mut formulas_to_upgrade: Vec<(UpgradeCandidate, Formula)> = Vec::new();
    let mut bottle_specs: Vec<BottleSpec> = Vec::new();
    let mut failed_fetches: Vec<(String, String)> = Vec::new(); // (name, reason)

    let mut index_refreshed = false;

    for candidate in upgradable {
        match sync.fetch_formula_cached(&candidate.name, None).await {
            Ok(formula) => {
                // Verify formula version matches expected version from index.
                // Compare effective versions (with _N revision suffix) so that
                // rebuilds of the same base version are handled correctly.
                let formula_effective = effective_version(&formula.version, formula.revision);
                if formula_effective != candidate.new_version {
                    // Try auto-updating the index once
                    if !index_refreshed {
                        println!(
                            "\n{} Version mismatch for {} (index: {}, formula: {}). Updating index...",
                            style("!").yellow(),
                            candidate.name,
                            candidate.new_version,
                            formula.version
                        );
                        match sync.sync_index(paths.index_db()).await {
                            Ok(_) => {
                                println!("  {} Index updated", style("✓").green());
                                index_refreshed = true;
                            }
                            Err(e) => {
                                failed_fetches.push((
                                    candidate.name.clone(),
                                    format!(
                                        "formula version mismatch and auto-update failed: {}",
                                        e
                                    ),
                                ));
                                continue;
                            }
                        }
                    }

                    // Re-fetch with fresh data
                    match sync.fetch_formula(&candidate.name).await {
                        Ok(fresh)
                            if effective_version(&fresh.version, fresh.revision)
                                == candidate.new_version
                                || fresh.version != formula.version =>
                        {
                            // Accept the fresh formula — proceed below
                            // (version may have changed, use fresh effective version)
                            let fresh_version = effective_version(&fresh.version, fresh.revision);
                            match fresh.bottle_for_platform(&platform) {
                                Some(bottle) => {
                                    bottle_specs.push(BottleSpec {
                                        name: candidate.name.clone(),
                                        version: fresh_version.clone(),
                                        platform: platform.clone(),
                                        url: bottle.url.clone(),
                                        sha256: bottle.sha256.clone(),
                                    });
                                    let mut updated = candidate;
                                    updated.new_version = fresh_version;
                                    formulas_to_upgrade.push((updated, fresh));
                                }
                                None => {
                                    failed_fetches.push((
                                        candidate.name.clone(),
                                        format!("no bottle for {}", platform),
                                    ));
                                }
                            }
                            continue;
                        }
                        _ => {
                            failed_fetches.push((
                                candidate.name.clone(),
                                format!(
                                    "formula version mismatch: index says {} but formula JSON has {}",
                                    candidate.new_version, formula.version
                                ),
                            ));
                            continue;
                        }
                    }
                }

                match formula.bottle_for_platform(&platform) {
                    Some(bottle) => {
                        bottle_specs.push(BottleSpec {
                            name: candidate.name.clone(),
                            version: candidate.new_version.clone(),
                            platform: platform.clone(),
                            url: bottle.url.clone(),
                            sha256: bottle.sha256.clone(),
                        });
                        formulas_to_upgrade.push((candidate, formula));
                    }
                    None => {
                        failed_fetches.push((
                            candidate.name.clone(),
                            format!("no bottle for {}", platform),
                        ));
                    }
                }
            }
            Err(e) => {
                failed_fetches.push((candidate.name.clone(), e.to_string()));
            }
        }
    }

    // Report formulas that couldn't be fetched
    if !failed_fetches.is_empty() {
        println!(
            "\n{} {} {} could not be upgraded:",
            style("!").yellow(),
            failed_fetches.len(),
            if failed_fetches.len() == 1 {
                "package"
            } else {
                "packages"
            }
        );
        for (name, reason) in &failed_fetches {
            println!(
                "  {} {} - {}",
                style("✗").red(),
                style(name).yellow(),
                style(reason).dim()
            );
        }
    }

    // Download and upgrade formulas (if any)
    let mut upgrade_errors: Vec<(String, String)> = Vec::new();

    if !bottle_specs.is_empty() {
        // Download all bottles in parallel
        println!(
            "\n{} {} packages...",
            style("Downloading").cyan(),
            bottle_specs.len()
        );

        let cache = DownloadCache::new(&paths.stout_dir);
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
            // Extract NEW version FIRST (before removing old)
            // This ensures we don't break the system if extraction fails
            let install_path = match extract_bottle(bottle_path, &paths.cellar) {
                Ok(path) => path,
                Err(e) => {
                    upgrade_errors
                        .push((candidate.name.clone(), format!("extraction failed: {}", e)));
                    continue;
                }
            };

            // Relocate Homebrew placeholders to actual paths
            if let Err(e) = relocate_bottle(&install_path, &paths.prefix) {
                // Non-fatal - package may still work, but log the issue
                warn!(
                    "Failed to relocate placeholders in {}: {}",
                    candidate.name, e
                );
            }

            // Now that extraction succeeded, unlink and remove OLD version
            let old_install_path = paths.package_path(&candidate.name, &candidate.old_version);
            if old_install_path.exists() {
                if let Err(e) = unlink_package(&old_install_path, &paths.prefix) {
                    upgrade_errors.push((
                        candidate.name.clone(),
                        format!("unlink old version failed: {}", e),
                    ));
                    // Try to clean up the newly extracted version
                    let _ = remove_package(&paths.cellar, &candidate.name, &candidate.new_version);
                    continue;
                }
                if let Err(e) =
                    remove_package(&paths.cellar, &candidate.name, &candidate.old_version)
                {
                    // Non-fatal - old files remain but new version is installed
                    warn!(
                        "Failed to remove old version {} of {}: {}",
                        candidate.old_version, candidate.name, e
                    );
                }
            }

            // Link new version
            if let Err(e) = link_package(&install_path, &paths.prefix) {
                upgrade_errors.push((candidate.name.clone(), format!("link failed: {}", e)));
                // Package is extracted but not linked - user can manually fix
                continue;
            }

            // Write receipt
            let runtime_deps: Vec<RuntimeDependency> = formula
                .runtime_deps()
                .iter()
                .filter_map(|dep| {
                    db.get_formula(dep)
                        .ok()
                        .flatten()
                        .map(|info| RuntimeDependency {
                            full_name: dep.clone(),
                            version: info.version,
                            revision: Some(info.revision),
                        })
                })
                .collect();

            let receipt = InstallReceipt::new_bottle(
                &formula.tap,
                candidate.explicitly_requested,
                runtime_deps,
            );
            if let Err(e) = write_receipt(&install_path, &receipt) {
                warn!("Failed to write receipt for {}: {}", candidate.name, e);
            }

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
    } // end if !bottle_specs.is_empty()

    // Save state
    installed.save(&paths)?;

    // Upgrade casks if any
    let mut casks_upgraded = 0usize;
    if !cask_upgrades.is_empty() {
        println!("\n{}...", style("Upgrading casks").cyan());

        let cask_cache_dir = paths.stout_dir.join("cache").join("casks");
        let tokens: Vec<String> = cask_upgrades.iter().map(|(t, _, _)| t.clone()).collect();
        let version_map: std::collections::HashMap<String, (String, String)> = cask_upgrades
            .iter()
            .map(|(t, old, new)| (t.clone(), (old.clone(), new.clone())))
            .collect();

        // Phase 1: Download all artifacts in parallel
        let downloads = crate::cli::install::download_casks(
            &tokens,
            &db,
            &cask_cache_dir,
            &config,
            &paths,
            true,
            false,
        )
        .await;

        // Report download errors
        for dl in &downloads {
            if let Some(e) = &dl.error {
                println!(
                    "  {} {} - {}",
                    style("✗").red(),
                    style(&dl.token).yellow(),
                    style(e).dim()
                );
                upgrade_errors.push((dl.token.clone(), e.clone()));
            }
        }

        // Phase 2: Install all casks sequentially
        let mut installed_casks =
            stout_cask::InstalledCasks::load(&cask_state_path).unwrap_or_default();

        for dl in downloads {
            if dl.error.is_some() {
                continue;
            }

            let (old_version, new_version) = version_map
                .get(&dl.token)
                .cloned()
                .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

            let install_options = stout_cask::CaskInstallOptions {
                force: true,
                dry_run: false,
                ..Default::default()
            };

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(CASK_INSTALL_TIMEOUT_SECS),
                tokio::task::spawn_blocking(move || {
                    stout_cask::install_artifact_sync(
                        &dl.cask.unwrap(),
                        &dl.artifact_path,
                        dl.artifact_type,
                        &install_options,
                    )
                }),
            )
            .await;

            match result {
                Err(_elapsed) => {
                    upgrade_errors.push((
                        dl.token,
                        format!("install timed out after {}s", CASK_INSTALL_TIMEOUT_SECS),
                    ));
                }
                Ok(Ok(Ok(install_path))) => {
                    casks_upgraded += 1;
                    let installed = stout_cask::InstalledCask {
                        version: new_version.clone(),
                        installed_at: stout_cask::now_timestamp(),
                        artifact_path: install_path,
                        auto_updates: false,
                        artifacts: vec![],
                    };
                    installed_casks.add(&dl.token, installed);

                    // Update Caskroom so sync sees the correct version
                    if let Err(e) = stout_install::cask_scan::register_cask_in_caskroom(
                        &paths.prefix,
                        &dl.token,
                        &new_version,
                    ) {
                        tracing::debug!("Failed to register {} in Caskroom: {}", dl.token, e);
                    }

                    println!(
                        "  {} {} {} → {}",
                        style("✓").green(),
                        dl.token,
                        style(&old_version).dim(),
                        style(&new_version).cyan()
                    );
                }
                Ok(Ok(Err(e))) => {
                    upgrade_errors.push((dl.token, format!("install failed: {}", e)));
                }
                Ok(Err(e)) => {
                    upgrade_errors.push((dl.token, format!("spawn error: {}", e)));
                }
            }
        }

        // Save state once
        if let Err(e) = installed_casks.save(&cask_state_path) {
            warn!("Failed to save cask state: {}", e);
        }
    }

    // Report any upgrade errors
    if !upgrade_errors.is_empty() {
        println!(
            "\n{} {} {} failed:",
            style("!").yellow(),
            upgrade_errors.len(),
            if upgrade_errors.len() == 1 {
                "package"
            } else {
                "packages"
            }
        );
        for (name, reason) in &upgrade_errors {
            println!(
                "  {} {} - {}",
                style("✗").red(),
                style(name).yellow(),
                style(reason).dim()
            );
        }
    }

    let elapsed = start.elapsed();
    let total = formulas_to_upgrade.len() + casks_upgraded + already_upgraded.len();
    print!(
        "\n{} {} {} in {:.1}s",
        style("Upgraded").green().bold(),
        total,
        if total == 1 { "package" } else { "packages" },
        elapsed.as_secs_f64()
    );
    if !failed_fetches.is_empty() || !upgrade_errors.is_empty() {
        let total_failed = failed_fetches.len() + upgrade_errors.len();
        print!(
            ", {} {} failed",
            style(total_failed).yellow(),
            if total_failed == 1 {
                "package"
            } else {
                "packages"
            }
        );
    }
    println!();

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

/// Build the effective version string used for comparison, incorporating the
/// Homebrew revision suffix (`_N`) when revision > 0.
///
/// This ensures that a rebuild of the same base version (e.g. `25.9.0_1`)
/// is correctly detected as newer than the previously installed `25.9.0`.
fn effective_version(version: &str, revision: u32) -> String {
    if revision > 0 {
        format!("{}_{}", version, revision)
    } else {
        version.to_string()
    }
}
