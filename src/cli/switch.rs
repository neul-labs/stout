//! Switch command - switch between installed versions of a package

use anyhow::{bail, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_state::{InstalledPackages, PackageHistory, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Formula to switch
    pub formula: String,

    /// Version to switch to
    pub version: String,

    /// Dry run - show what would be done
    #[arg(long, short = 'n')]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    // Check if package is installed
    if !installed.is_installed(&args.formula) {
        bail!("{} is not installed", args.formula);
    }

    let current = installed.get(&args.formula).ok_or_else(|| {
        anyhow::anyhow!(
            "package '{}' is installed but not found in state",
            args.formula
        )
    })?;
    let current_version = &current.version;

    // Check if target version is in cellar
    let available_versions = paths.installed_versions(&args.formula);

    if available_versions.is_empty() {
        bail!("No versions of {} found in Cellar", args.formula);
    }

    if !available_versions.contains(&args.version) {
        println!(
            "{} Version {} is not installed",
            style("Error:").red().bold(),
            style(&args.version).yellow()
        );
        println!();
        println!("Available versions in Cellar:");
        for v in &available_versions {
            let marker = if v == current_version {
                style("*").green().bold().to_string()
            } else {
                " ".to_string()
            };
            println!("  {} {}", marker, v);
        }
        bail!("Version {} not found", args.version);
    }

    if args.version == *current_version {
        println!(
            "{} is already at version {}",
            style(&args.formula).cyan(),
            style(&args.version).green()
        );
        return Ok(());
    }

    println!(
        "{} Switching {} {} -> {}",
        style("==>").blue().bold(),
        style(&args.formula).cyan().bold(),
        style(current_version).dim(),
        style(&args.version).green()
    );

    if args.dry_run {
        println!();
        println!("{} Dry run - no changes made", style("Note:").yellow());
        return Ok(());
    }

    // Switch symlinks to the target version
    switch_version(&args.formula, &args.version, &paths)?;

    // Update installed state
    let mut installed = InstalledPackages::load(&paths)?;
    let pkg = installed.get(&args.formula).ok_or_else(|| {
        anyhow::anyhow!(
            "package '{}' is installed but not found in state",
            args.formula
        )
    })?;
    let requested = pkg.requested;
    let deps = pkg.dependencies.clone();
    installed.add_with_deps(&args.formula, &args.version, 0, requested, deps);
    installed.save(&paths)?;

    // Record history
    let mut history = PackageHistory::load(&paths)?;
    // Determine if this is upgrade or downgrade
    if version_cmp(&args.version, current_version) > 0 {
        history.record_upgrade(&args.formula, &args.version, 0, current_version, 0);
    } else {
        history.record_downgrade(&args.formula, &args.version, 0, current_version, 0);
    }
    history.save(&paths)?;

    println!(
        "{} Switched {} to version {}",
        style("✓").green().bold(),
        style(&args.formula).cyan(),
        style(&args.version).green()
    );

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

/// Simple version comparison (returns -1, 0, or 1)
fn version_cmp(a: &str, b: &str) -> i32 {
    let parse_parts = |v: &str| -> Vec<u64> {
        v.split(|c: char| !c.is_ascii_digit())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    };

    let parts_a = parse_parts(a);
    let parts_b = parse_parts(b);

    for i in 0..parts_a.len().max(parts_b.len()) {
        let pa = parts_a.get(i).copied().unwrap_or(0);
        let pb = parts_b.get(i).copied().unwrap_or(0);
        if pa < pb {
            return -1;
        }
        if pa > pb {
            return 1;
        }
    }
    0
}
