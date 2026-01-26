//! Reinstall command - uninstall and reinstall a package

use anyhow::{bail, Context, Result};
use stout_fetch::{BottleSpec, DownloadCache, DownloadClient, ProgressReporter};
use stout_index::{Database, IndexSync};
use stout_install::{
    extract_bottle, link_package, unlink_package, write_receipt, BuildConfig, InstallReceipt,
    RuntimeDependency, SourceBuilder,
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
    let platform = detect_platform();

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

        // Unlink old version
        let old_install_path = paths.cellar.join(name).join(&old_pkg.version);
        if old_install_path.exists() {
            println!("  {} Unlinking old version...", style("•").dim());
            let _ = unlink_package(&old_install_path, &paths.prefix);
        }

        // Determine if we're building from source
        let use_source = args.build_from_source || formula.bottle_for_platform(&platform).is_none();

        let install_path = if use_source {
            // Build from source
            let source = formula.urls.stable.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No source URL available for {}", name)
            })?;

            println!("  {} Building from source...", style("•").dim());

            let build_config = BuildConfig {
                source_url: source.url.clone(),
                sha256: source.sha256.clone(),
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

        let receipt = if use_source {
            InstallReceipt::new_source(&formula.tap, old_pkg.requested, runtime_deps)
        } else {
            InstallReceipt::new_bottle(&formula.tap, old_pkg.requested, runtime_deps)
        };
        write_receipt(&install_path, &receipt)?;

        // Update installed state (preserve requested status)
        installed.add(name, &formula.version, formula.revision, old_pkg.requested);

        // Remove old version from cellar if different
        if old_pkg.version != formula.version && old_install_path.exists() {
            let _ = std::fs::remove_dir_all(&old_install_path);
        }

        println!(
            "{} Reinstalled {} {}",
            style("✓").green(),
            name,
            formula.version
        );
    }

    // Save state
    installed.save(&paths)?;

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
