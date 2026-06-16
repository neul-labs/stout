//! Cleanup command for removing old files and cache
//!
//! Compatible with `brew cleanup` - removes stale lock files and outdated
//! downloads, and removes old versions of installed packages.

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use std::path::Path;
use stout_cask::InstalledCasks;
use stout_fetch::DownloadCache;
use stout_state::{InstalledPackages, Paths};

/// Default cleanup age in days (same as Homebrew's HOMEBREW_CLEANUP_MAX_AGE_DAYS)
const DEFAULT_CLEANUP_DAYS: u64 = 120;

#[derive(ClapArgs)]
pub struct Args {
    /// Specific formulas/casks to clean up (default: all)
    pub formulas: Vec<String>,

    /// Remove all cache files older than specified days (use 0 for all)
    #[arg(long, value_name = "DAYS")]
    pub prune: Option<u64>,

    /// Scrub cache, removing downloads even for latest versions
    /// (still keeps downloads for installed packages)
    #[arg(long, short = 's')]
    pub scrub: bool,

    /// Only show what would be removed without actually removing
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Only prune symlinks and directories from prefix
    #[arg(long)]
    pub prune_prefix: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;
    let mut total_freed: u64 = 0;
    let mut items_removed: usize = 0;

    println!("\n{}...", style("Cleaning up").cyan());

    // Handle --prune-prefix separately
    if args.prune_prefix {
        let freed = prune_prefix_only(&paths, args.dry_run)?;
        total_freed += freed.0;
        items_removed += freed.1;
    } else {
        // Determine max age for cleanup
        let max_age_days = args.prune.unwrap_or(DEFAULT_CLEANUP_DAYS);
        let max_age_secs = max_age_days * 24 * 60 * 60;

        // Clean download cache
        let cache = DownloadCache::new(&paths.stout_dir);

        if max_age_days == 0 || args.scrub {
            // Remove all downloads (--prune=0 or --scrub)
            let freed = clean_all_downloads(&paths, &installed, args.scrub, args.dry_run)?;
            total_freed += freed.0;
            items_removed += freed.1;
        } else {
            // Remove old downloads based on age
            if args.dry_run {
                let (size, count) = preview_old_downloads(&paths, max_age_secs)?;
                total_freed += size;
                items_removed += count;
            } else {
                let freed = cache
                    .clean(max_age_secs)
                    .context("Failed to clean download cache")?;
                if freed > 0 {
                    println!(
                        "  {} Removed stale downloads: {}",
                        style("✓").green(),
                        format_bytes(freed)
                    );
                    total_freed += freed;
                }
            }
        }

        // Clean cask artifact cache
        let cask_state_path = paths.stout_dir.join("casks.json");
        let installed_casks = InstalledCasks::load(&cask_state_path).unwrap_or_default();
        let cask_cache_dir = paths.stout_dir.join("cache").join("casks");

        if max_age_days == 0 || args.scrub {
            let freed = clean_all_cask_artifacts(
                &cask_cache_dir,
                &installed_casks,
                args.scrub,
                args.dry_run,
            )?;
            total_freed += freed.0;
            items_removed += freed.1;
        } else if args.dry_run {
            let (size, count) = preview_old_cask_artifacts(&cask_cache_dir, max_age_secs)?;
            total_freed += size;
            items_removed += count;
        } else {
            let freed = clean_old_cask_artifacts(&cask_cache_dir, max_age_secs)?;
            if freed > 0 {
                println!(
                    "  {} Removed stale cask artifacts: {}",
                    style("✓").green(),
                    format_bytes(freed)
                );
                total_freed += freed;
            }
        }

        // Remove old versions from Cellar (always done by brew cleanup)
        let freed = scrub_old_versions(&paths, &args.formulas, args.dry_run)?;
        total_freed += freed.0;
        items_removed += freed.1;

        // Clean formula/cask JSON cache if scrubbing
        if args.scrub {
            let freed = clean_json_cache(&paths, args.dry_run)?;
            total_freed += freed.0;
            items_removed += freed.1;
        }
    }

    // Summary
    if args.dry_run {
        if total_freed > 0 || items_removed > 0 {
            println!(
                "\n{} Would free {} ({} items)",
                style("Dry run:").yellow(),
                format_bytes(total_freed),
                items_removed
            );
        } else {
            println!("\n{}", style("Nothing to clean up.").dim());
        }
    } else if total_freed > 0 {
        println!(
            "\n{} Freed {}",
            style("Cleaned up").green().bold(),
            format_bytes(total_freed)
        );
    } else {
        println!("\n{}", style("Nothing to clean up.").dim());
    }

    Ok(())
}

/// Clean all downloaded bottles
/// If scrub is false, keeps downloads for installed packages
fn clean_all_downloads(
    paths: &Paths,
    installed: &InstalledPackages,
    scrub: bool,
    dry_run: bool,
) -> Result<(u64, usize)> {
    let downloads_dir = paths.stout_dir.join("downloads");
    if !downloads_dir.exists() {
        return Ok((0, 0));
    }

    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(&downloads_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let filename = entry.file_name().to_string_lossy().to_string();

            // If not scrubbing, skip downloads for installed packages
            if !scrub {
                // Check if this file belongs to an installed package
                // Filename format: name-version-platform.tar.gz
                // Since package names can contain hyphens, check each installed package
                let is_for_installed = installed
                    .names()
                    .any(|pkg_name| is_download_for_package(&filename, pkg_name));
                if is_for_installed {
                    continue;
                }
            }

            let size = entry.metadata()?.len();
            total_size += size;
            count += 1;

            if dry_run {
                println!(
                    "  {} {} ({})",
                    style("Would remove:").yellow(),
                    filename,
                    format_bytes(size)
                );
            } else {
                std::fs::remove_file(entry.path())?;
                println!(
                    "  {} Removed {} ({})",
                    style("✓").green(),
                    filename,
                    format_bytes(size)
                );
            }
        }
    }

    Ok((total_size, count))
}

/// Check if a download filename is for a specific package
/// Filename format: name-version-platform.tar.gz
/// Package names can contain hyphens, so we check if the filename starts with "name-"
fn is_download_for_package(filename: &str, pkg_name: &str) -> bool {
    // Remove .tar.gz extension if present
    let base_name = filename.strip_suffix(".tar.gz").unwrap_or(filename);

    // Check if the filename starts with "package-" (with hyphen to avoid partial matches)
    base_name.starts_with(&format!("{}-", pkg_name))
}

/// Check if a cask artifact filename is for a specific cask
/// Filename format: name.<ext> (e.g. firefox.dmg, slack.zip)
/// Cask names can contain hyphens, so we check if the filename starts with "name."
fn is_cask_artifact_for_cask(filename: &str, cask_name: &str) -> bool {
    // Check if the filename starts with "cask." (with dot to avoid partial matches)
    filename.starts_with(&format!("{}.", cask_name))
}

/// Preview what old downloads would be removed
fn preview_old_downloads(paths: &Paths, max_age_secs: u64) -> Result<(u64, usize)> {
    let downloads_dir = paths.stout_dir.join("downloads");
    if !downloads_dir.exists() {
        return Ok((0, 0));
    }

    let now = std::time::SystemTime::now();
    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(&downloads_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            if let Ok(modified) = metadata.modified() {
                if let Ok(age) = now.duration_since(modified) {
                    if age.as_secs() > max_age_secs {
                        let size = metadata.len();
                        total_size += size;
                        count += 1;
                        println!(
                            "  {} {} ({}, {} days old)",
                            style("Would remove:").yellow(),
                            entry.file_name().to_string_lossy(),
                            format_bytes(size),
                            age.as_secs() / 86400
                        );
                    }
                }
            }
        }
    }

    Ok((total_size, count))
}

/// Clean all cask artifact downloads
/// If scrub is false, keeps artifacts for installed casks
fn clean_all_cask_artifacts(
    cache_dir: &Path,
    installed_casks: &InstalledCasks,
    scrub: bool,
    dry_run: bool,
) -> Result<(u64, usize)> {
    if !cache_dir.exists() {
        return Ok((0, 0));
    }

    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }

        let filename = entry.file_name().to_string_lossy().to_string();

        // If not scrubbing, skip artifacts for installed casks
        if !scrub {
            // Check if this artifact belongs to an installed cask
            // Filename format: name.<ext> (e.g. firefox.dmg, slack.zip)
            let is_for_installed = installed_casks
                .tokens()
                .any(|cask_name| is_cask_artifact_for_cask(&filename, cask_name));
            if is_for_installed {
                continue;
            }
        }

        let size = entry.metadata()?.len();
        total_size += size;
        count += 1;

        if dry_run {
            println!(
                "  {} {} ({})",
                style("Would remove:").yellow(),
                filename,
                format_bytes(size)
            );
        } else {
            std::fs::remove_file(entry.path())?;
            println!(
                "  {} Removed {} ({})",
                style("✓").green(),
                filename,
                format_bytes(size)
            );
        }
    }

    Ok((total_size, count))
}

/// Preview what old cask artifacts would be removed
fn preview_old_cask_artifacts(cache_dir: &Path, max_age_secs: u64) -> Result<(u64, usize)> {
    if !cache_dir.exists() {
        return Ok((0, 0));
    }

    let now = std::time::SystemTime::now();
    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if !metadata.is_file() {
            continue;
        }

        if let Ok(modified) = metadata.modified() {
            if let Ok(age) = now.duration_since(modified) {
                if age.as_secs() > max_age_secs {
                    let size = metadata.len();
                    total_size += size;
                    count += 1;
                    println!(
                        "  {} {} ({}, {} days old)",
                        style("Would remove:").yellow(),
                        entry.file_name().to_string_lossy(),
                        format_bytes(size),
                        age.as_secs() / 86400
                    );
                }
            }
        }
    }

    Ok((total_size, count))
}

/// Remove old cask artifacts based on age
fn clean_old_cask_artifacts(cache_dir: &Path, max_age_secs: u64) -> Result<u64> {
    if !cache_dir.exists() {
        return Ok(0);
    }

    let now = std::time::SystemTime::now();
    let mut freed = 0u64;

    for entry in std::fs::read_dir(cache_dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if !metadata.is_file() {
            continue;
        }

        if let Ok(modified) = metadata.modified() {
            if let Ok(age) = now.duration_since(modified) {
                if age.as_secs() > max_age_secs {
                    freed += metadata.len();
                    std::fs::remove_file(entry.path())?;
                }
            }
        }
    }

    Ok(freed)
}

/// Clean formula/cask JSON cache
fn clean_json_cache(paths: &Paths, dry_run: bool) -> Result<(u64, usize)> {
    let mut total_size = 0u64;
    let mut count = 0usize;

    for subdir in &["formulas", "casks"] {
        let cache_dir = paths.stout_dir.join(subdir);
        if !cache_dir.exists() {
            continue;
        }

        let result = clean_directory(&cache_dir, dry_run)?;
        total_size += result.0;
        count += result.1;
    }

    if count > 0 && !dry_run {
        println!(
            "  {} Cleared JSON cache: {}",
            style("✓").green(),
            format_bytes(total_size)
        );
    }

    Ok((total_size, count))
}

/// Scrub old versions from Cellar (keep only latest)
/// If formulas is non-empty, only clean those specific packages
fn scrub_old_versions(paths: &Paths, formulas: &[String], dry_run: bool) -> Result<(u64, usize)> {
    let installed = InstalledPackages::load(paths)?;
    let mut total_size = 0u64;
    let mut count = 0usize;

    if !paths.cellar.exists() {
        return Ok((0, 0));
    }

    for entry in std::fs::read_dir(&paths.cellar)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let pkg_name = entry.file_name().to_string_lossy().to_string();

        // If specific formulas requested, skip others
        if !formulas.is_empty() && !formulas.contains(&pkg_name) {
            continue;
        }

        // Get the current installed version
        let current_version = installed.get(&pkg_name).map(|p| p.version.clone());

        // List all versions in cellar
        let pkg_dir = entry.path();
        for version_entry in std::fs::read_dir(&pkg_dir)? {
            let version_entry = version_entry?;
            if !version_entry.file_type()?.is_dir() {
                continue;
            }

            let version = version_entry.file_name().to_string_lossy().to_string();

            // Skip current version
            if current_version.as_ref() == Some(&version) {
                continue;
            }

            // This is an old version, remove it
            let size = dir_size(&version_entry.path())?;
            total_size += size;
            count += 1;

            if dry_run {
                println!(
                    "  {} {}/{} ({})",
                    style("Would remove:").yellow(),
                    pkg_name,
                    version,
                    format_bytes(size)
                );
            } else {
                std::fs::remove_dir_all(version_entry.path())?;
                println!(
                    "  {} Removed old version: {}/{} ({})",
                    style("✓").green(),
                    pkg_name,
                    version,
                    format_bytes(size)
                );
            }
        }

        // Remove package directory if empty (no versions left)
        if !dry_run && pkg_dir.read_dir()?.next().is_none() {
            std::fs::remove_dir(&pkg_dir)?;
        }
    }

    Ok((total_size, count))
}

/// Prune only symlinks and directories from prefix
fn prune_prefix_only(paths: &Paths, dry_run: bool) -> Result<(u64, usize)> {
    let mut total_size = 0u64;
    let mut count = 0usize;

    // Check for broken symlinks in prefix directories
    for subdir in &["bin", "sbin", "lib", "include", "share", "etc", "opt"] {
        let dir = paths.prefix.join(subdir);
        if !dir.exists() {
            continue;
        }

        let result = prune_broken_symlinks(&dir, dry_run)?;
        total_size += result.0;
        count += result.1;
    }

    if count > 0 && !dry_run {
        println!("  {} Pruned {} broken symlinks", style("✓").green(), count);
    }

    Ok((total_size, count))
}

/// Remove broken symlinks from a directory
fn prune_broken_symlinks(dir: &Path, dry_run: bool) -> Result<(u64, usize)> {
    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_symlink() {
            // Check if symlink target exists
            if !path.exists() {
                count += 1;
                if dry_run {
                    println!(
                        "  {} {} (broken symlink)",
                        style("Would remove:").yellow(),
                        path.display()
                    );
                } else {
                    std::fs::remove_file(&path)?;
                }
            }
        } else if path.is_dir() {
            let result = prune_broken_symlinks(&path, dry_run)?;
            total_size += result.0;
            count += result.1;

            // Remove empty directories
            if !dry_run {
                if let Ok(mut entries) = std::fs::read_dir(&path) {
                    if entries.next().is_none() {
                        let _ = std::fs::remove_dir(&path);
                    }
                }
            }
        }
    }

    Ok((total_size, count))
}

/// Clean all files in a directory
fn clean_directory(dir: &Path, dry_run: bool) -> Result<(u64, usize)> {
    let mut total_size = 0u64;
    let mut count = 0usize;

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            let size = metadata.len();
            total_size += size;
            count += 1;

            if !dry_run {
                std::fs::remove_file(entry.path())?;
            }
        } else if metadata.is_dir() {
            let result = clean_directory(&entry.path(), dry_run)?;
            total_size += result.0;
            count += result.1;

            if !dry_run {
                let _ = std::fs::remove_dir(entry.path());
            }
        }
    }

    Ok((total_size, count))
}

/// Calculate total size of a directory
fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;

    if path.is_file() {
        return Ok(std::fs::metadata(path)?.len());
    }

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let metadata = entry.metadata()?;

        if metadata.is_file() {
            total += metadata.len();
        } else if metadata.is_dir() {
            total += dir_size(&entry.path())?;
        }
    }

    Ok(total)
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
