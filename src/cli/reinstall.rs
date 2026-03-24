//! Reinstall command - uninstall and reinstall a package

use anyhow::{bail, Context, Result};
use stout_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use stout_index::{Database, IndexSync};
use stout_install::{
    extract_bottle, link_package, unlink_package, write_receipt, BuildConfig, HeadBuildConfig,
    HeadBuilder, InstallReceipt, RuntimeDependency, SourceBuilder,
};
use stout_state::{Config, InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;
use std::sync::Arc;

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to reinstall
    pub formulas: Vec<String>,

    /// Build from source instead of using bottles
    #[arg(long, short = 's')]
    pub build_from_source: bool,

    /// Reinstall from HEAD (latest git commit)
    #[arg(long = "HEAD", short = 'H')]
    pub head: bool,

    /// Keep downloaded bottles after installation
    #[arg(long)]
    pub keep_bottles: bool,
}

pub async fn run(args: Args) -> Result<()> {
    if args.formulas.is_empty() {
        bail!("No formulas specified");
    }

    let paths = Paths::default();
    paths.ensure_dirs()?;
    let config = Config::load(&paths)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        bail!("Index not initialized. Run 'stout update' first.");
    }

    let mut installed = InstalledPackages::load(&paths)?;
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    // Detect platform
    let platform = super::detect_platform();

    for name in &args.formulas {
        // Check if installed
        let old_pkg = installed.get(name)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("{} is not installed, use 'stout install' instead", name))?;

        println!(
            "\n{} Reinstalling {} {}",
            style("==>").blue().bold(),
            style(name).cyan(),
            style(&old_pkg.version).dim()
        );

        // Fetch formula
        let formula = sync
            .fetch_formula_cached(name, None)
            .await
            .context(format!("Failed to fetch formula {}", name))?;

        // Warn if formula version differs from installed version
        if formula.version != old_pkg.version && !old_pkg.is_head_install() {
            println!(
                "  {} Formula version {} differs from installed {} - will reinstall to newer version",
                style("!").yellow(),
                formula.version,
                old_pkg.version
            );
        }

        // Determine reinstall type
        let reinstall_as_head =
            args.head || (old_pkg.is_head_install() && !args.build_from_source);

        // Unlink old version
        let old_install_path = paths.cellar.join(name).join(&old_pkg.version);
        if old_install_path.exists() {
            println!("  {} Unlinking old version...", style("•").dim());
            let _ = unlink_package(&old_install_path, &paths.prefix);
        }

        let install_path = if reinstall_as_head {
            // Build from HEAD
            let head_url = formula.urls.head.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "No HEAD URL available for {}. Use -s for stable source builds.",
                    name
                )
            })?;

            println!(
                "  {} Building from HEAD...",
                style("•").dim()
            );

            let head_config = HeadBuildConfig {
                git_url: head_url.url.clone(),
                branch: head_url.branch.clone().unwrap_or_else(|| "master".to_string()),
                name: name.clone(),
                prefix: paths.prefix.clone(),
                cellar: paths.cellar.clone(),
                jobs: None,
                cc: None,
                cxx: None,
            };

            let work_dir = paths.stout_dir.join("build").join(name);
            let builder = HeadBuilder::new(head_config, &work_dir);

            let result = builder.build().await.context(format!(
                "Failed to build {} from HEAD",
                name
            ))?;

            // Cleanup build directory
            let _ = std::fs::remove_dir_all(&work_dir);

            // Check if SHA changed
            if let Some(ref old_sha) = old_pkg.head_sha {
                if old_sha == &result.commit_sha {
                    println!(
                        "  {} Already at latest HEAD ({})",
                        style("•").dim(),
                        result.short_sha
                    );
                } else {
                    println!(
                        "  {} Updated {} → {}",
                        style("•").dim(),
                        old_pkg.short_sha().unwrap_or("?"),
                        result.short_sha
                    );
                }
            }

            // Update state with new HEAD SHA
            installed.add_head(
                name,
                &result.short_sha,
                &result.commit_sha,
                old_pkg.requested,
                formula.runtime_deps().to_vec(),
            );

            result.install_path
        } else if args.build_from_source || formula.bottle_for_platform(&platform).is_none() {
            // Build from source
            let source = formula.urls.stable.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No source URL available for {}", name)
            })?;

            println!("  {} Building from source...", style("•").dim());

            let build_config = BuildConfig {
                source_url: source.url.clone(),
                sha256: source.sha256.clone().unwrap_or_default(),
                name: name.clone(),
                version: formula.version.clone(),
                prefix: paths.prefix.clone(),
                cellar: paths.cellar.clone(),
                build_deps: formula.build_deps().to_vec(),
                jobs: None,
                cc: None,
                cxx: None,
            };

            let work_dir = paths.stout_dir.join("build").join(name);
            let builder = SourceBuilder::new(build_config, &work_dir);

            let result = builder.build().await.context(format!(
                "Failed to build {} from source",
                name
            ))?;

            // Cleanup build directory
            let _ = std::fs::remove_dir_all(&work_dir);

            result.install_path
        } else {
            // Download and extract bottle
            let bottle = formula.bottle_for_platform(&platform)
                .expect("bottle_for_platform returned None after None check");

            println!("  {} Downloading bottle...", style("•").dim());

            let cache = DownloadCache::new(&paths.stout_dir);
            let client = DownloadClient::new(cache, 1)?;
            let progress = Arc::new(ProgressReporter::new());

            let bottle_spec = BottleSpec {
                name: name.clone(),
                version: formula.version.clone(),
                platform: platform.clone(),
                url: bottle.url.clone(),
                sha256: bottle.sha256.clone(),
            };

            let bottle_paths = client
                .download_bottles(vec![bottle_spec], progress)
                .await
                .context("Failed to download bottle")?;

            let bottle_path = &bottle_paths[0];

            println!("  {} Extracting...", style("•").dim());
            let install_path = extract_bottle(bottle_path, &paths.cellar)?;

            // Cleanup bottle if not keeping
            if !args.keep_bottles {
                let _ = std::fs::remove_file(bottle_path);
            }

            install_path
        };

        // Get the actual installed version from the install path
        // (includes revision suffix like 25.8.1_1)
        let installed_version = install_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&formula.version)
            .to_string();

        // Link new version
        println!("  {} Linking...", style("•").dim());
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

        let receipt = if reinstall_as_head {
            InstallReceipt::new_source(&formula.tap, old_pkg.requested, runtime_deps)
        } else if args.build_from_source || formula.bottle_for_platform(&platform).is_none() {
            InstallReceipt::new_source(&formula.tap, old_pkg.requested, runtime_deps)
        } else {
            InstallReceipt::new_bottle(&formula.tap, old_pkg.requested, runtime_deps)
        };
        write_receipt(&install_path, &receipt)?;

        // Update installed state
        if reinstall_as_head {
            // HEAD install - state already updated in the HEAD build branch
        } else {
            // Stable install - preserve requested status
            // Use installed_version which includes revision suffix
            installed.add(name, &installed_version, formula.revision, old_pkg.requested);
        }

        // Remove old version from cellar if different
        if old_pkg.version != installed_version && old_install_path.exists() {
            let _ = std::fs::remove_dir_all(&old_install_path);
        }

        // Print success message
        if reinstall_as_head {
            println!(
                "{} Reinstalled {} (HEAD)",
                style("✓").green(),
                name,
            );
        } else {
            println!(
                "{} Reinstalled {} {}",
                style("✓").green(),
                name,
                installed_version
            );
        }
    }

    // Save state
    installed.save(&paths)?;

    Ok(())
}

