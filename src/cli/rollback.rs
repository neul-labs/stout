//! Rollback command - revert a package to its previous version

use anyhow::{bail, Context, Result};
use stout_index::{Database, IndexSync};
use stout_state::{Config, InstalledPackages, PackageHistory, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to rollback
    pub formula: String,

    /// Rollback to a specific version (default: previous version)
    #[arg(long, short = 'v')]
    pub version: Option<String>,

    /// Dry run - show what would be done
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;
    let installed = InstalledPackages::load(&paths)?;
    let history = PackageHistory::load(&paths)?;

    // Check if package is installed
    if !installed.is_installed(&args.formula) {
        bail!("{} is not installed", args.formula);
    }

    let current = installed.get(&args.formula)
        .with_context(|| format!("package '{}' is installed but not found in state", args.formula))?;
    let current_version = &current.version;

    // Determine target version
    let target_version = if let Some(v) = &args.version {
        v.clone()
    } else {
        // Get previous version from history
        let prev = history.get_previous(&args.formula)
            .ok_or_else(|| anyhow::anyhow!(
                "No previous version found for {}. Use --version to specify a version.",
                args.formula
            ))?;
        prev.version.clone()
    };

    if target_version == *current_version {
        println!(
            "{} is already at version {}",
            style(&args.formula).cyan(),
            style(&target_version).green()
        );
        return Ok(());
    }

    println!(
        "{} Rolling back {} {} -> {}",
        style("==>").blue().bold(),
        style(&args.formula).cyan().bold(),
        style(current_version).red(),
        style(&target_version).green()
    );

    if args.dry_run {
        println!();
        println!("{} Dry run - no changes made", style("Note:").yellow());
        return Ok(());
    }

    // Check if target version is available in cellar (already downloaded)
    let target_path = paths.package_path(&args.formula, &target_version);
    if target_path.exists() {
        println!(
            "  {} Found {} in Cellar, switching...",
            style("•").dim(),
            style(&target_version).green()
        );

        // Switch symlinks to the target version
        switch_version(&args.formula, &target_version, &paths)?;

        // Update installed state
        let mut installed = InstalledPackages::load(&paths)?;
        let pkg = installed.get(&args.formula)
            .with_context(|| format!("package '{}' is installed but not found in state", args.formula))?;
        let requested = pkg.requested;
        let deps = pkg.dependencies.clone();
        installed.add_with_deps(&args.formula, &target_version, 0, requested, deps);
        installed.save(&paths)?;

        // Record history
        let mut history = PackageHistory::load(&paths)?;
        history.record_downgrade(&args.formula, &target_version, 0, current_version, 0);
        history.save(&paths)?;

        println!(
            "{} Rolled back {} to {}",
            style("✓").green().bold(),
            style(&args.formula).cyan(),
            style(&target_version).green()
        );
    } else {
        // Need to download the target version
        println!(
            "  {} Version {} not in Cellar, downloading...",
            style("•").dim(),
            style(&target_version).yellow()
        );

        // For now, we inform the user they need to install the specific version
        // A full implementation would integrate with stout-install to download
        // the specific version bottle.
        println!();
        println!(
            "{} To rollback to {}, run:",
            style("Hint:").yellow(),
            style(&target_version).green()
        );
        println!(
            "  stout uninstall {} && stout install {}@{}",
            args.formula, args.formula, target_version
        );
        println!();
        println!(
            "{} Note: Version pinning (package@version) requires the version to be available in the index.",
            style("Note:").dim()
        );

        bail!(
            "Rollback requires downloading version {}. See hint above.",
            target_version
        );
    }

    Ok(())
}

/// Switch symlinks to a different version
fn switch_version(name: &str, version: &str, paths: &Paths) -> Result<()> {
    let pkg_path = paths.package_path(name, version);
    if !pkg_path.exists() {
        bail!("Package path does not exist: {}", pkg_path.display());
    }

    // Get the bin, lib, share, etc. directories
    let subdirs = ["bin", "lib", "share", "include", "etc", "man"];

    for subdir in &subdirs {
        let src_dir = pkg_path.join(subdir);
        if !src_dir.exists() {
            continue;
        }

        let dest_dir = paths.prefix.join(subdir);

        // Remove existing symlinks for this package
        if dest_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&dest_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_symlink() {
                        if let Ok(target) = std::fs::read_link(&path) {
                            // Check if symlink points to any version of this package
                            let target_str = target.to_string_lossy();
                            if target_str.contains(&format!("/{}/", name)) {
                                let _ = std::fs::remove_file(&path);
                            }
                        }
                    }
                }
            }
        }

        // Create new symlinks
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let src_path = entry.path();
                let file_name = entry.file_name();
                let dest_path = dest_dir.join(&file_name);

                // Create parent dir if needed
                if let Some(parent) = dest_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }

                // Create symlink
                #[cfg(unix)]
                {
                    let _ = std::os::unix::fs::symlink(&src_path, &dest_path);
                }
            }
        }
    }

    Ok(())
}
