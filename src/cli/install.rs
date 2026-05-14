//! Install command

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use console::style;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

const CASK_INSTALL_TIMEOUT_SECS: u64 = 300;
use stout_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use stout_index::{Database, Formula, FormulaInfo, IndexSync};
use stout_install::{
    link_package, scan_cellar_package, write_receipt, BottleInfo, BuildConfig, HeadBuildConfig,
    HeadBuilder, InstallReceipt, ParallelInstaller, RuntimeDependency, SourceBuilder,
};
use stout_resolve::{DependencyGraph, InstallPlan, InstallStep};
use stout_state::{Config, InstalledPackages, Paths, TapManager};

#[derive(ClapArgs)]
pub struct Args {
    /// Packages to install (formulas or casks)
    #[arg(value_name = "PACKAGES")]
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

    /// Install from HEAD (latest git commit, implies --build-from-source)
    #[arg(long = "HEAD", short = 'H')]
    pub head: bool,

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

    /// Force download even if package already exists in Cellar
    #[arg(long)]
    pub force: bool,

    /// Treat all packages as casks
    #[arg(long)]
    pub cask: bool,

    /// Treat all packages as formulas
    #[arg(long, conflicts_with = "cask")]
    pub formula: bool,

    /// Skip checksum verification for casks
    #[arg(long)]
    pub no_verify: bool,

    /// Custom application directory for casks
    #[arg(long)]
    pub appdir: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let start = Instant::now();

    if args.formulas.is_empty() {
        bail!("No packages specified");
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

    // Categorize packages as formulas or casks
    let mut formulas: Vec<String> = Vec::new();
    let mut casks: Vec<String> = Vec::new();

    for name in &args.formulas {
        if args.cask {
            // Explicitly marked as cask
            if db.get_cask(name)?.is_none() {
                let suggestions = db.find_similar_casks(name, 3)?;
                eprintln!(
                    "\n{} cask '{}' not found",
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
            casks.push(name.clone());
        } else if args.formula {
            // Explicitly marked as formula
            // Check if this is a tap formula first
            let is_tap_formula = name.matches('/').count() == 2;

            if is_tap_formula {
                // Tap formulas aren't in the database
                formulas.push(name.clone());
            } else if db.get_formula(name)?.is_none() {
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
            } else {
                formulas.push(name.clone());
            }
        } else {
            // Check if this is a tap formula (format: user/tap/formula)
            let is_tap_formula = name.matches('/').count() == 2;

            if is_tap_formula {
                // Tap formulas aren't in the database, so treat as formula without validation
                formulas.push(name.clone());
            } else {
                let is_formula = db.get_formula(name)?.is_some();
                let is_cask = db.get_cask(name)?.is_some();

                match (is_formula, is_cask) {
                    (true, true) => {
                        // Name conflict - ask user
                        eprintln!(
                            "\n{} '{}' exists as both a formula and a cask.",
                            style("!").yellow(),
                            name
                        );
                        eprintln!(
                            "  {} Use '--cask' to install the cask, or specify explicitly:",
                            style("Tip:").dim()
                        );
                        eprintln!(
                            "    {} stout install {}       # formula (default)",
                            style("$").dim(),
                            name
                        );
                        eprintln!(
                            "    {} stout install --cask {} # cask",
                            style("$").dim(),
                            name
                        );
                        std::process::exit(1);
                    }
                    (true, false) => formulas.push(name.clone()),
                    (false, true) => casks.push(name.clone()),
                    (false, false) => {
                        // Don't fail immediately - try to find in taps during install phase
                        // For now, just mark as a regular formula to check later
                        formulas.push(name.clone());
                    }
                }
            }
        }
    }

    // Install formulas
    let mut total: usize = 0;
    if !formulas.is_empty() {
        total += install_formulas(&args, &formulas, &db, &config, &paths).await?;
    }

    // Install casks
    if !casks.is_empty() {
        total += install_casks(&args, &casks, &db, &config, &paths).await?;
    }

    let elapsed = start.elapsed();
    if total > 0 {
        println!(
            "\n{} {} {} in {:.1}s",
            style("Installed").green().bold(),
            total,
            if total == 1 { "package" } else { "packages" },
            elapsed.as_secs_f64()
        );
    }

    Ok(())
}

/// Recursively copy a directory tree, preserving permissions.
fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
            if let Ok(meta) = src_path.metadata() {
                let _ = std::fs::set_permissions(&dst_path, meta.permissions());
            }
        }
    }
    Ok(())
}

/// Install tap formula contents into the cellar.
///
/// Copies the full archive tree into `libexec/{formula_name}/` and creates
/// relative symlinks in `bin/` for each executable at the root of libexec.
/// This matches what Homebrew does for formulas that use `libexec.install Dir["*"]`
/// followed by `bin.install_symlink`, and ensures that `$FindBin::Bin`-relative
/// paths (e.g. Perl's `use lib "$FindBin::Bin/lib"`) resolve correctly.
fn install_tap_formula_contents(
    content_root: &std::path::Path,
    install_path: &std::path::Path,
    formula_name: &str,
) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let libexec_dir = install_path.join("libexec").join(formula_name);
    copy_dir_all(content_root, &libexec_dir)?;

    let bin_dir = install_path.join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    // Symlink root-level executables from bin/ → ../libexec/{formula_name}/{file}
    for entry in std::fs::read_dir(&libexec_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Ok(meta) = path.metadata() {
                if meta.permissions().mode() & 0o111 != 0 {
                    let file_name = entry.file_name();
                    let rel = std::path::PathBuf::from(format!(
                        "../libexec/{}/{}",
                        formula_name,
                        file_name.to_string_lossy()
                    ));
                    let link = bin_dir.join(&file_name);
                    if link.exists() || link.symlink_metadata().is_ok() {
                        std::fs::remove_file(&link)?;
                    }
                    std::os::unix::fs::symlink(&rel, &link)?;
                }
            }
        }
    }

    Ok(())
}

async fn install_tap_formulas(
    args: &Args,
    formulas: &[String],
    config: &Config,
    paths: &Paths,
    installed: &mut InstalledPackages,
) -> Result<usize> {
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let platform = super::detect_platform();

    println!(
        "\n{} {} tap {}...",
        style("Installing").cyan(),
        formulas.len(),
        if formulas.len() == 1 {
            "formula"
        } else {
            "formulas"
        }
    );

    for name in formulas {
        println!("  {} {}", style("Installing").yellow(), name);

        let formula = sync
            .fetch_formula(name)
            .await
            .context(format!("Failed to fetch tap formula {}", name))?;

        // For tap formulas, extract just the formula name (last component) for cellar paths
        // This keeps the cellar structure consistent with Homebrew (package name only, not full tap path)
        let formula_name = name.split('/').next_back().unwrap_or(name.as_str());

        // For tap formulas, treat pre-built binaries (from stable URL) as if they were bottles
        // Extract and link them directly instead of building from source
        if let Some(stable_url) = &formula.urls.stable {
            // Use a safe name for the cache (slashes replaced with dashes)
            let safe_formula_name = formula_name.replace('/', "-");

            let cache = DownloadCache::new(&paths.stout_dir);
            let client = DownloadClient::new(cache, 1)?;

            let bottle_spec = BottleSpec {
                name: safe_formula_name.clone(),
                version: formula.version.clone(),
                platform: platform.clone(),
                url: stable_url.url.clone(),
                sha256: stable_url.sha256.clone().unwrap_or_default(),
            };

            let bottle_paths = client
                .download_bottles(vec![bottle_spec], Arc::new(ProgressReporter::new()))
                .await
                .context(format!("Failed to download {}", name))?;

            if let Some(bottle_path) = bottle_paths.first() {
                // For tap formulas, the archive may not be in Homebrew bottle format
                // Extract as a simple tar archive
                let extract_dir = paths.stout_dir.join("tap-extract").join(&safe_formula_name);
                std::fs::create_dir_all(&extract_dir)?;

                // Extract the archive
                let output = std::process::Command::new("tar")
                    .arg("-xzf")
                    .arg(bottle_path)
                    .current_dir(&extract_dir)
                    .output()?;

                if !output.status.success() {
                    bail!(
                        "Failed to extract {}: {}",
                        formula_name,
                        String::from_utf8_lossy(&output.stderr)
                    );
                }

                // Create install directory (use formula_name without tap path)
                let install_path = paths.cellar.join(formula_name).join(&formula.version);
                std::fs::create_dir_all(&install_path)?;

                // Handle archives that extract to a single top-level directory
                let content_root = {
                    let entries: Vec<_> = std::fs::read_dir(&extract_dir)?
                        .filter_map(|e| e.ok())
                        .collect();
                    if entries.len() == 1 && entries[0].path().is_dir() {
                        entries[0].path()
                    } else {
                        extract_dir.clone()
                    }
                };

                install_tap_formula_contents(&content_root, &install_path, formula_name)?;

                // Link the installation to the prefix
                link_package(&install_path, &paths.prefix)?;

                let receipt = InstallReceipt::new_bottle(&formula.tap, true, Vec::new());
                write_receipt(&install_path, &receipt)?;

                installed.add_with_deps(
                    name,
                    &formula.version,
                    formula.revision,
                    true,
                    formula.runtime_deps().to_vec(),
                );

                println!("  {} {}", style("✓").green(), name,);

                // Cleanup extract directory
                let _ = std::fs::remove_dir_all(&extract_dir);

                // Cleanup bottle
                if !args.keep_bottles && bottle_path.exists() {
                    let _ = std::fs::remove_file(bottle_path);
                }
            }
        } else {
            bail!("No installation method available for {}", name);
        }
    }

    installed.save(paths)?;
    Ok(formulas.len())
}

async fn install_formulas(
    args: &Args,
    formulas: &[String],
    db: &Database,
    config: &Config,
    paths: &Paths,
) -> Result<usize> {
    // Separate tap formulas (format: user/tap/formula) from regular formulas
    let mut regular_formulas = Vec::new();
    let mut tap_formulas = Vec::new();

    for formula in formulas {
        if formula.matches('/').count() == 2 {
            tap_formulas.push(formula.clone());
        } else {
            regular_formulas.push(formula.clone());
        }
    }

    // Load installed packages
    let mut installed = InstalledPackages::load(paths)?;
    let mut installed_count: usize = 0;

    // For tap formulas, we can't use the dependency graph since they're not in the database.
    // Just fetch them directly and install without resolving dependencies.
    if !tap_formulas.is_empty() {
        installed_count +=
            install_tap_formulas(args, &tap_formulas, config, paths, &mut installed).await?;
    }

    // If there are no regular formulas, we're done
    if regular_formulas.is_empty() {
        return Ok(installed_count);
    }

    // Check if any regular formulas are missing from the database - they might be in taps
    let tap_manager = TapManager::load(paths)?;

    // Formulas found via CDN fallback that aren't in the local DB.
    // Keyed by bare formula name; used as a fallback in the from_graph get_info closure
    // so the plan is built correctly even when the DB doesn't have the entry.
    let mut cdn_formula_info: HashMap<String, Formula> = HashMap::new();

    let mut final_formulas = Vec::new();
    for formula_name in regular_formulas {
        if db.get_formula(&formula_name)?.is_none() {
            // Formula not in database, might be in a tap
            let mut found_in_tap = false;

            // Try each tap to find the formula
            for tap in tap_manager.list() {
                // Parse the tap name (e.g., "SyntheticAutonomicMind/homebrew-SAM" -> "user", "repo")
                let tap_parts: Vec<&str> = tap.name.split('/').collect();
                if tap_parts.len() == 2 {
                    let tap_user = tap_parts[0];
                    let tap_repo = if tap_parts[1].starts_with("homebrew-") {
                        &tap_parts[1][9..] // Remove "homebrew-" prefix
                    } else {
                        tap_parts[1]
                    };

                    let full_name = format!("{}/{}/{}", tap_user, tap_repo, formula_name);
                    // Use the main sync — fetch_tap_formula fetches from GitHub directly
                    // so the base URL doesn't matter for 3-part names.
                    let main_sync = IndexSync::with_security_policy(
                        Some(&config.index.base_url),
                        &paths.stout_dir,
                        config.security.to_security_policy(),
                    )?;
                    if main_sync.fetch_formula(&full_name).await.is_ok() {
                        tap_formulas.push(full_name);
                        found_in_tap = true;
                        break;
                    }
                }
            }

            if !found_in_tap {
                // Last resort: try the CDN index and then the Homebrew API.
                let index_sync = IndexSync::with_security_policy(
                    Some(&config.index.base_url),
                    &paths.stout_dir,
                    config.security.to_security_policy(),
                )?;
                match index_sync.fetch_formula(&formula_name).await {
                    Ok(formula) => {
                        let platform = super::detect_platform();
                        if formula.bottle_for_platform(&platform).is_some() {
                            // Has a bottle — use the regular bottle install path.
                            // Store the formula so from_graph's get_info closure can find it.
                            cdn_formula_info.insert(formula_name.clone(), formula);
                            final_formulas.push(formula_name);
                        } else {
                            // No bottle — treat as a tap formula (downloads stable URL directly).
                            tap_formulas.push(formula_name);
                        }
                    }
                    Err(_) => {
                        eprintln!(
                            "\n{} formula '{}' not found in index or any configured tap",
                            style("error:").red().bold(),
                            formula_name
                        );
                        bail!("Formula '{}' not found", formula_name);
                    }
                }
            }
        } else {
            final_formulas.push(formula_name);
        }
    }

    let formula_refs: Vec<&str> = final_formulas.iter().map(|s| s.as_str()).collect();

    // Install tap formulas (both originally-3-part and CDN-discovered-no-bottle)
    if !tap_formulas.is_empty() {
        installed_count +=
            install_tap_formulas(args, &tap_formulas, config, paths, &mut installed).await?;
    }

    // If there are no more regular formulas after separation, we're done
    if formula_refs.is_empty() {
        return Ok(installed_count);
    }

    // Build dependency graph
    println!("\n{}...", style("Resolving dependencies").cyan());

    let graph = DependencyGraph::build_from_db(db, &formula_refs, false)?;

    // Create install plan
    let mut plan = InstallPlan::from_graph(
        &graph,
        &formula_refs,
        |name| {
            db.get_formula(name).ok().flatten().or_else(|| {
                cdn_formula_info.get(name).map(|f| FormulaInfo {
                    name: f.name.clone(),
                    version: f.version.clone(),
                    revision: f.revision,
                    desc: f.desc.clone(),
                    homepage: f.homepage.clone(),
                    license: f.license.clone(),
                    tap: f.tap.clone(),
                    deprecated: f.flags.deprecated,
                    disabled: f.flags.disabled,
                    has_bottle: !f.bottles.is_empty(),
                    json_hash: None,
                })
            })
        },
        |name| installed.is_installed(name),
    )?;

    // Verify "already installed" packages actually exist in the Cellar.
    // If the Cellar was externally modified (e.g. brew uninstall or manual rm),
    // the state file is stale — remove from state and re-add to the install plan.
    let mut stale_packages: Vec<String> = Vec::new();
    for name in &plan.already_installed {
        if let Some(pkg) = installed.get(name) {
            let install_path = paths.cellar.join(name).join(&pkg.version);
            if !install_path.exists() {
                stale_packages.push(name.clone());
            }
        }
    }

    for name in &stale_packages {
        plan.already_installed.remove(name);
        if let Some(info) = db.get_formula(name).ok().flatten() {
            let is_dep = !plan.requested.contains(name);
            if is_dep {
                plan.dependencies.insert(name.clone());
            }
            plan.steps.push(InstallStep {
                name: name.clone(),
                version: info.version.clone(),
                is_dependency: is_dep,
            });
        }
        installed.remove(name);
        println!(
            "  {} {} {}",
            style("→").yellow(),
            name,
            style("(stale state, will reinstall)").dim()
        );
    }

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
        return Ok(installed_count);
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(installed_count);
    }

    // Targeted Cellar pre-check: detect packages already in Cellar at target version
    let mut cellar_registered: Vec<String> = Vec::new();
    if !args.force {
        for step in plan.new_packages() {
            if let Some(cellar_pkg) = scan_cellar_package(&paths.cellar, &step.name)? {
                if cellar_pkg.version == step.version {
                    // Already in Cellar at target version — register without downloading.
                    // Preserve existing installed_by if already tracked (stout didn't do the install).
                    if let Some(existing) = installed.get(&step.name) {
                        let existing = existing.clone();
                        installed.add_imported(
                            &step.name,
                            &step.version,
                            0,
                            !step.is_dependency,
                            &existing.installed_by,
                            &existing.installed_at,
                            existing.dependencies.clone(),
                        );
                    } else {
                        // Not tracked yet — mark as "brew" since stout didn't install it
                        installed.add_imported(
                            &step.name,
                            &step.version,
                            0,
                            !step.is_dependency,
                            "brew",
                            &stout_install::cellar::timestamp_to_iso(0),
                            Vec::new(),
                        );
                    }
                    cellar_registered.push(step.name.clone());
                    println!(
                        "  {} {} {} {}",
                        style("✓").green(),
                        step.name,
                        step.version,
                        style("(already installed, registered in stout)").dim()
                    );
                }
            }
        }
    }

    // Detect platform
    let platform = super::detect_platform();

    // Fetch full formula data and categorize by installation method
    println!("\n{}...", style("Fetching formula data").cyan());
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let mut bottle_installs: Vec<(InstallStep, Formula)> = Vec::new();
    let mut source_installs: Vec<(InstallStep, Formula)> = Vec::new();
    let mut head_installs: Vec<(InstallStep, Formula)> = Vec::new();
    let mut bottle_specs: Vec<BottleSpec> = Vec::new();

    // HEAD implies build-from-source
    let build_from_source = args.build_from_source || args.head;

    for step in plan.new_packages() {
        // Skip packages already registered from Cellar pre-check
        if cellar_registered.contains(&step.name) {
            continue;
        }

        let formula = sync
            .fetch_formula_cached(&step.name, None)
            .await
            .context(format!("Failed to fetch formula {}", step.name))?;

        // Verify formula version matches expected version from index
        // Verify formula version matches expected version from index.
        // This catches stale formula JSON that doesn't match the index.
        // Skip version check for HEAD builds since HEAD doesn't have a version.
        if !args.head && formula.version != step.version {
            // Auto-update the index and retry
            println!(
                "\n{} Version mismatch for {} (index: {}, formula: {}). Updating index...",
                style("!").yellow(),
                step.name,
                step.version,
                formula.version
            );

            let update_sync = IndexSync::with_security_policy(
                Some(&config.index.base_url),
                &paths.stout_dir,
                config.security.to_security_policy(),
            )?;
            update_sync
                .sync_index(paths.index_db())
                .await
                .context("Auto-update failed. Try running 'stout update' manually.")?;

            // Re-open the database with the fresh index
            let fresh_db =
                Database::open(paths.index_db()).context("Failed to re-open index after update")?;

            // Re-fetch the formula — the cached JSON may also be stale
            let fresh_formula = sync
                .fetch_formula(&step.name)
                .await
                .context(format!("Failed to re-fetch formula {}", step.name))?;

            if fresh_formula.version
                != fresh_db
                    .get_formula(&step.name)?
                    .map(|i| i.version)
                    .unwrap_or_default()
            {
                bail!(
                    "Formula version mismatch persists for {} after update. \
                     Index: {}, formula JSON: {}. This may be a transient issue — try again shortly.",
                    step.name,
                    step.version,
                    fresh_formula.version
                );
            }

            println!(
                "  {} Index updated. Continuing with {} {}",
                style("✓").green(),
                step.name,
                fresh_formula.version
            );

            // Use the fresh formula for the rest of the flow —
            // update step version and replace formula.
            // Note: step is cloned, so we rebuild it.
            let fresh_step = InstallStep {
                name: step.name.clone(),
                version: fresh_formula.version.clone(),
                is_dependency: step.is_dependency,
            };

            if args.head && !fresh_step.is_dependency {
                if fresh_formula.urls.head.is_none() {
                    bail!(
                        "No HEAD URL available for {}. Use -s for stable source builds.",
                        fresh_step.name
                    );
                }
                head_installs.push((fresh_step, fresh_formula));
                continue;
            }

            let use_source =
                build_from_source || fresh_formula.bottle_for_platform(&platform).is_none();

            if use_source {
                if fresh_formula.urls.stable.is_none() {
                    bail!("No source URL available for {}", fresh_step.name);
                }
                source_installs.push((fresh_step, fresh_formula));
            } else {
                let bottle = fresh_formula
                    .bottle_for_platform(&platform)
                    .expect("bottle_for_platform returned None after None check");
                bottle_specs.push(BottleSpec {
                    name: fresh_step.name.clone(),
                    version: fresh_step.version.clone(),
                    platform: platform.clone(),
                    url: bottle.url.clone(),
                    sha256: bottle.sha256.clone(),
                });
                bottle_installs.push((fresh_step, fresh_formula));
            }
            continue;
        }

        // HEAD builds: only for explicitly requested packages (not dependencies)
        if args.head && !step.is_dependency {
            if formula.urls.head.is_none() {
                bail!(
                    "No HEAD URL available for {}. Use -s for stable source builds.",
                    step.name
                );
            }
            head_installs.push((step.clone(), formula));
            continue;
        }

        let use_source = build_from_source || formula.bottle_for_platform(&platform).is_none();

        if use_source {
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
            let bottle = formula
                .bottle_for_platform(&platform)
                .expect("bottle_for_platform returned None after None check");
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

        let bottle_infos: Vec<BottleInfo> = bottle_installs
            .iter()
            .zip(bottle_paths.iter())
            .map(|((step, _), bottle_path)| BottleInfo {
                name: step.name.clone(),
                bottle_path: bottle_path.clone(),
            })
            .collect();

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

            let receipt =
                InstallReceipt::new_bottle(&formula.tap, !step.is_dependency, runtime_deps);
            write_receipt(&result.install_path, &receipt)?;

            installed.add_with_deps(
                &step.name,
                &step.version,
                formula.revision,
                !step.is_dependency,
                formula.runtime_deps().to_vec(),
            );
            if let Some(bottle) = formula.bottle_for_platform(&platform) {
                if let Some(pkg) = installed.packages.get_mut(&step.name) {
                    pkg.bottle_sha256 = Some(bottle.sha256.clone());
                }
            }
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
            let source = formula
                .urls
                .stable
                .as_ref()
                .expect("stable URL should exist for source builds");

            println!(
                "  {} {} {} (from source)",
                style("Building").yellow(),
                step.name,
                step.version
            );

            let build_config = BuildConfig {
                source_url: source.url.clone(),
                sha256: source.sha256.clone().unwrap_or_default(),
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

            let result = builder
                .build()
                .await
                .context(format!("Failed to build {} from source", step.name))?;

            link_package(&result.install_path, &paths.prefix)?;

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

            let receipt =
                InstallReceipt::new_source(&formula.tap, !step.is_dependency, runtime_deps);
            write_receipt(&result.install_path, &receipt)?;

            installed.add_with_deps(
                &step.name,
                &step.version,
                formula.revision,
                !step.is_dependency,
                formula.runtime_deps().to_vec(),
            );

            let _ = std::fs::remove_dir_all(&work_dir);

            println!("  {} {} {}", style("✓").green(), step.name, step.version);
        }
    }

    // Build from HEAD
    if !head_installs.is_empty() {
        println!("\n{}...", style("Building from HEAD").cyan());

        for (step, formula) in &head_installs {
            let head_url = formula.urls.head.as_ref().expect("head URL should exist");

            println!("  {} {} (from HEAD)", style("Building").yellow(), step.name);

            let head_config = HeadBuildConfig {
                git_url: head_url.url.clone(),
                branch: head_url
                    .branch
                    .clone()
                    .unwrap_or_else(|| "master".to_string()),
                name: step.name.clone(),
                prefix: paths.prefix.clone(),
                cellar: paths.cellar.clone(),
                jobs: args.jobs,
                cc: args.cc.clone(),
                cxx: args.cxx.clone(),
            };

            let work_dir = paths.stout_dir.join("build").join(&step.name);
            let builder = HeadBuilder::new(head_config, &work_dir);

            let result = builder
                .build()
                .await
                .context(format!("Failed to build {} from HEAD", step.name))?;

            link_package(&result.install_path, &paths.prefix)?;

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

            let receipt =
                InstallReceipt::new_source(&formula.tap, !step.is_dependency, runtime_deps);
            write_receipt(&result.install_path, &receipt)?;

            installed.add_head(
                &step.name,
                &result.short_sha,
                &result.commit_sha,
                !step.is_dependency,
                formula.runtime_deps().to_vec(),
            );

            let _ = std::fs::remove_dir_all(&work_dir);

            println!(
                "  {} {} {}",
                style("✓").green(),
                step.name,
                result.short_sha
            );
        }
    }

    // Save state
    installed.save(paths)?;

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

    // Check patchelf on Linux and warn if not found
    #[cfg(target_os = "linux")]
    {
        let patchelf_available = std::process::Command::new("patchelf")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !patchelf_available && !bottle_installs.is_empty() {
            println!();
            println!(
                "  {} {}",
                style("⚠").yellow().bold(),
                style("patchelf not found - binaries may not run correctly")
                    .yellow()
                    .bold()
            );
            println!(
                "    {}",
                style("Install patchelf to fix ELF binaries: sudo apt install patchelf").dim()
            );
            println!(
                "    {}",
                style("Then reinstall packages: stout reinstall <package>").dim()
            );
        }
    }

    installed_count += bottle_installs.len() + source_installs.len() + head_installs.len();
    Ok(installed_count)
}

async fn install_casks(
    args: &Args,
    casks: &[String],
    db: &Database,
    config: &Config,
    paths: &Paths,
) -> Result<usize> {
    println!(
        "\n{} {} {}...",
        style("Installing").cyan(),
        casks.len(),
        if casks.len() == 1 { "cask" } else { "casks" }
    );

    let cask_cache_dir = paths.stout_dir.join("cache").join("casks");
    let cask_state_path = paths.stout_dir.join("casks.json");

    // Phase 1: Download all artifacts in parallel
    let downloads = download_casks(
        casks,
        db,
        &cask_cache_dir,
        config,
        paths,
        args.force,
        args.dry_run,
    )
    .await;
    let mut errors = collect_download_errors(&downloads);

    if args.dry_run {
        for dl in &downloads {
            if dl.error.is_none() {
                println!("  {} {} [dry-run]", style("✓").green(), dl.token);
            }
        }
        if !errors.is_empty() {
            for (token, e) in &errors {
                eprintln!(
                    "  {} {} - {}",
                    style("✗").red(),
                    style(token).yellow(),
                    style(e).dim()
                );
            }
            bail!("Failed to download {} cask(s)", errors.len());
        }
        return Ok(0);
    }

    // Phase 2: Install all casks sequentially (terminal access for sudo, etc.)
    let mut installed_casks =
        stout_cask::InstalledCasks::load(&cask_state_path).unwrap_or_default();
    let mut installed_count: usize = 0;

    for dl in downloads {
        if let Some(e) = &dl.error {
            println!(
                "  {} {} - {}",
                style("✗").red(),
                style(&dl.token).yellow(),
                style(e).dim()
            );
            errors.push((dl.token, e.clone()));
            continue;
        }

        println!(
            "  {} {} {}",
            style("→").cyan(),
            dl.token,
            style(format!("({})", dl.artifact_type.extension())).dim(),
        );
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        let install_options = stout_cask::CaskInstallOptions {
            force: args.force,
            no_verify: args.no_verify,
            appdir: args.appdir.as_deref().map(std::path::PathBuf::from),
            dry_run: false,
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
                println!(
                    "  {} {} - {}",
                    style("✗").red(),
                    style(&dl.token).yellow(),
                    style(format!(
                        "install timed out after {}s",
                        CASK_INSTALL_TIMEOUT_SECS
                    ))
                    .dim()
                );
                errors.push((
                    dl.token,
                    format!("install timed out after {}s", CASK_INSTALL_TIMEOUT_SECS),
                ));
            }
            Ok(Ok(Ok(install_path))) => {
                let version = db
                    .get_cask(&dl.token)
                    .ok()
                    .flatten()
                    .map(|i| i.version)
                    .unwrap_or_else(|| "unknown".to_string());

                let installed = stout_cask::InstalledCask {
                    version: version.clone(),
                    installed_at: stout_cask::now_timestamp(),
                    artifact_path: install_path,
                    auto_updates: false,
                    artifacts: vec![],
                };
                installed_casks.add(&dl.token, installed);
                installed_count += 1;

                // Register in Caskroom so sync sees the correct version
                if let Err(e) = stout_install::cask_scan::register_cask_in_caskroom(
                    &paths.prefix,
                    &dl.token,
                    &version,
                ) {
                    tracing::debug!("Failed to register {} in Caskroom: {}", dl.token, e);
                }

                println!("  {} {}", style("✓").green(), dl.token);
            }
            Ok(Ok(Err(e))) => {
                println!(
                    "  {} {} - {}",
                    style("✗").red(),
                    style(&dl.token).yellow(),
                    style(&e).dim()
                );
                errors.push((dl.token, format!("install failed: {}", e)));
            }
            Ok(Err(e)) => {
                println!(
                    "  {} {} - {}",
                    style("✗").red(),
                    style(&dl.token).yellow(),
                    style(&e).dim()
                );
                errors.push((dl.token, format!("spawn error: {}", e)));
            }
        }
    }

    // Save state once
    if let Err(e) = installed_casks.save(&cask_state_path) {
        tracing::warn!("Failed to save cask state: {}", e);
    }

    if !errors.is_empty() {
        bail!("Failed to install {} cask(s)", errors.len());
    }

    Ok(installed_count)
}

/// Result of downloading a single cask artifact.
pub(crate) struct CaskDownload {
    pub token: String,
    pub cask: Option<stout_index::Cask>,
    pub artifact_path: std::path::PathBuf,
    pub artifact_type: stout_cask::ArtifactType,
    pub error: Option<String>,
}

/// Download cask artifacts in parallel. Returns results in original order.
/// Skips the install step — the caller handles installation sequentially.
pub(crate) async fn download_casks(
    tokens: &[String],
    db: &Database,
    cache_dir: &std::path::Path,
    config: &Config,
    paths: &Paths,
    _force: bool,
    _dry_run: bool,
) -> Vec<CaskDownload> {
    use futures_util::stream::{FuturesUnordered, StreamExt};
    use stout_cask;

    let index_base_url = config.index.base_url.clone();
    let security_policy = config.security.to_security_policy();
    let stout_dir = paths.stout_dir.clone();
    let cache_dir = cache_dir.to_path_buf();

    let mut futures: FuturesUnordered<_> = tokens
        .iter()
        .enumerate()
        .map(|(idx, token)| {
            let token = token.to_string();
            let cache_dir = cache_dir.clone();
            let index_base_url = index_base_url.clone();
            let stout_dir = stout_dir.clone();
            let security_policy = security_policy.clone();

            async move {
                // Verify cask exists in database
                match db.get_cask(&token) {
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: None,
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type: stout_cask::ArtifactType::Dmg,
                                error: Some("cask not found in database".to_string()),
                            },
                        )
                    }
                    Err(e) => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: None,
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type: stout_cask::ArtifactType::Dmg,
                                error: Some(format!("database error: {}", e)),
                            },
                        )
                    }
                }

                let sync = match IndexSync::with_security_policy(
                    Some(&index_base_url),
                    &stout_dir,
                    security_policy,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: None,
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type: stout_cask::ArtifactType::Dmg,
                                error: Some(format!("failed to create sync: {}", e)),
                            },
                        )
                    }
                };

                let cask = match sync.fetch_cask_cached(&token, None).await {
                    Ok(c) => c,
                    Err(e) => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: None,
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type: stout_cask::ArtifactType::Dmg,
                                error: Some(format!("failed to fetch cask: {}", e)),
                            },
                        )
                    }
                };

                let url = match cask.download_url() {
                    Some(u) => u.to_string(),
                    None => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: None,
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type: stout_cask::ArtifactType::Dmg,
                                error: Some("no download URL".to_string()),
                            },
                        )
                    }
                };

                let mut artifact_type = stout_cask::detect_artifact_type_from_cask(&cask, &url);
                let sha256 = cask.sha256.as_str();

                let mut artifact_path = match stout_cask::download_cask_artifact(
                    &url,
                    &cache_dir,
                    &token,
                    sha256,
                    artifact_type,
                )
                .await
                {
                    Ok(p) => p,
                    Err(e) => {
                        return (
                            idx,
                            CaskDownload {
                                token: token.clone(),
                                cask: Some(cask),
                                artifact_path: std::path::PathBuf::new(),
                                artifact_type,
                                error: Some(format!("download failed: {}", e)),
                            },
                        )
                    }
                };

                // Re-check actual file type via magic bytes
                if let Some(real_type) = stout_cask::detect_artifact_type_from_magic(&artifact_path)
                {
                    if real_type != artifact_type {
                        let correct_path = artifact_path
                            .parent()
                            .unwrap_or(&artifact_path)
                            .join(format!("{}.{}", token, real_type.extension()));
                        if let Err(e) = std::fs::rename(&artifact_path, &correct_path) {
                            tracing::warn!(
                                "Could not rename {} to {}: {}",
                                artifact_path.display(),
                                correct_path.display(),
                                e
                            );
                        } else {
                            tracing::debug!(
                                "Corrected artifact type for {} from {} to {}",
                                token,
                                artifact_type.extension(),
                                real_type.extension()
                            );
                            artifact_path = correct_path;
                            artifact_type = real_type;
                        }
                    }
                }

                (
                    idx,
                    CaskDownload {
                        token,
                        cask: Some(cask),
                        artifact_path,
                        artifact_type,
                        error: None,
                    },
                )
            }
        })
        .collect();

    // Collect results preserving original order
    let mut results: Vec<CaskDownload> = (0..tokens.len())
        .map(|i| CaskDownload {
            token: tokens[i].clone(),
            cask: None,
            artifact_path: std::path::PathBuf::new(),
            artifact_type: stout_cask::ArtifactType::Dmg,
            error: Some("download not completed".to_string()),
        })
        .collect();

    while let Some((idx, dl)) = futures.next().await {
        results[idx] = dl;
    }

    results
}

/// Collect errors from download results for reporting.
pub(crate) fn collect_download_errors(downloads: &[CaskDownload]) -> Vec<(String, String)> {
    downloads
        .iter()
        .filter_map(|dl| dl.error.as_ref().map(|e| (dl.token.clone(), e.clone())))
        .collect()
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
