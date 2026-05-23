//! Doctor command - check system health

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use std::io::Write;
#[cfg(target_os = "macos")]
use std::path::Path;
use stout_index::Database;
use stout_install::cellar::scan_cellar;
use stout_install::{relocate_bottle, scan_unrelocated_files};
use stout_state::{Config, InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Automatically fix issues that can be repaired
    #[arg(long)]
    pub fix: bool,
}

/// Results from fixing upgrade-related issues
#[derive(Debug, Default)]
pub struct UpgradeFixResults {
    pub placeholders_fixed: usize,
    pub signatures_fixed: usize,
    pub packages_reinstalled: Vec<String>,
    #[cfg(target_os = "macos")]
    pub packages_with_broken_sigs: Vec<(String, usize)>, // (pkg_name, broken_count)
}

/// Check and optionally fix issues: unresolved placeholders and invalid code signatures.
/// If packages is empty, checks/fixes all packages in Cellar (used by doctor).
/// If packages is non-empty, checks/fixes only those packages (used by upgrade).
/// If should_fix is false, only scans and counts issues without modifying files.
/// If should_fix is true, applies fixes and reinstalls packages with unfixable signatures.
pub async fn fix_upgrade_issues(
    packages: Vec<String>,
    paths: &Paths,
    should_fix: bool,
) -> Result<UpgradeFixResults> {
    let mut results = UpgradeFixResults::default();

    let cellar_packages = scan_cellar(&paths.cellar)?;

    // Determine which packages to check
    let target_packages: Vec<_> = if packages.is_empty() {
        // Check all packages if none specified
        cellar_packages.iter().map(|p| p.name.clone()).collect()
    } else {
        packages
    };

    // Check/fix unresolved placeholders
    for pkg_name in &target_packages {
        if let Some(pkg) = cellar_packages.iter().find(|p| &p.name == pkg_name) {
            if should_fix {
                if let Ok(count) = relocate_bottle(&pkg.path, &paths.prefix) {
                    results.placeholders_fixed += count;
                }
            } else {
                // When not fixing, scan for unresolved placeholders without modifying them
                if let Ok(unrelocated) = scan_unrelocated_files(&pkg.path) {
                    results.placeholders_fixed += unrelocated.len();
                }
            }
        }
    }

    // Check/fix code signatures on macOS and reinstall if necessary
    #[cfg(target_os = "macos")]
    {
        let mut packages_to_reinstall = Vec::new();

        for pkg_name in &target_packages {
            if let Some(pkg) = cellar_packages.iter().find(|p| &p.name == pkg_name) {
                if should_fix {
                    let (fixed, still_broken) = check_and_fix_signatures(&pkg.path);
                    results.signatures_fixed += fixed;
                    if !still_broken.is_empty() {
                        results
                            .packages_with_broken_sigs
                            .push((pkg_name.clone(), still_broken.len()));
                        packages_to_reinstall.push(pkg_name.clone());
                    }
                } else {
                    // When not fixing, only scan for broken signatures
                    let broken = find_broken_signatures(&pkg.path);
                    if !broken.is_empty() {
                        results
                            .packages_with_broken_sigs
                            .push((pkg_name.clone(), broken.len()));
                    }
                }
            }
        }

        // Reinstall packages that still have broken signatures (only when fixing)
        if should_fix && !packages_to_reinstall.is_empty() {
            let reinstall_args = crate::cli::reinstall::Args {
                formulas: packages_to_reinstall.clone(),
                build_from_source: false,
                head: false,
                keep_bottles: false,
            };
            match crate::cli::reinstall::run(reinstall_args).await {
                Ok(_) => {
                    results.packages_reinstalled = packages_to_reinstall;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    Ok(results)
}

/// Find all Mach-O binary files in a package directory.
#[cfg(target_os = "macos")]
fn walk_macho_files(pkg_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    use walkdir::WalkDir;

    WalkDir::new(pkg_path)
        .into_iter()
        .filter_entry(|e| e.file_name().to_str().is_some_and(|n| !n.starts_with('.')))
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let metadata = std::fs::symlink_metadata(entry.path()).ok()?;
            if !metadata.is_file() {
                return None;
            }
            if !is_macho_file(entry.path()) {
                return None;
            }
            Some(entry.path().to_path_buf())
        })
        .collect()
}

/// Find all files with invalid code signatures in a package directory.
#[cfg(target_os = "macos")]
fn find_broken_signatures(pkg_path: &std::path::Path) -> Vec<std::path::PathBuf> {
    walk_macho_files(pkg_path)
        .into_iter()
        .filter(|path| !verify_codesign(path))
        .collect()
}

/// Check and fix code signatures for a package directory.
/// Returns (fixed_count, still_broken_files)
#[cfg(target_os = "macos")]
fn check_and_fix_signatures(pkg_path: &std::path::Path) -> (usize, Vec<std::path::PathBuf>) {
    let broken = find_broken_signatures(pkg_path);

    let mut fixed = 0usize;
    let mut still_broken = Vec::new();

    for file in broken {
        if resign_file(&file) {
            fixed += 1;
        } else {
            still_broken.push(file);
        }
    }

    (fixed, still_broken)
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    println!("\n{}", style("stout doctor").cyan().bold());
    println!("{}\n", style("Checking system health...").dim());

    let mut issues = 0;

    // Check stout directory
    print!("  Checking stout directory... ");
    if paths.stout_dir.exists() {
        println!("{}", style("✓").green());
    } else {
        println!(
            "{} (will be created on first use)",
            style("missing").yellow()
        );
    }

    // Check config
    print!("  Checking configuration... ");
    match Config::load(&paths) {
        Ok(_) => println!("{}", style("✓").green()),
        Err(e) => {
            println!("{}", style("✗").red());
            println!("    {}", style(format!("Error: {}", e)).red());
            issues += 1;
        }
    }

    // Check index
    print!("  Checking formula index... ");
    match Database::open(paths.index_db()) {
        Ok(db) => {
            if db.is_initialized().unwrap_or(false) {
                let count = db.formula_count().unwrap_or(0);
                println!("{} ({} formulas)", style("✓").green(), count);
            } else {
                println!("{}", style("not initialized").yellow());
                println!("    {}", style("Run 'stout update' to initialize").dim());
            }
        }
        Err(e) => {
            println!("{}", style("✗").red());
            println!("    {}", style(format!("Error: {}", e)).red());
            issues += 1;
        }
    }

    // Check Homebrew prefix
    print!("  Checking Homebrew prefix... ");
    if paths.prefix.exists() {
        println!("{} ({})", style("✓").green(), paths.prefix.display());
    } else {
        println!("{}", style("not found").yellow());
        println!(
            "    {}",
            style(format!("Expected at: {}", paths.prefix.display())).dim()
        );
    }

    // Check Cellar
    print!("  Checking Cellar... ");
    if paths.cellar.exists() {
        let count = std::fs::read_dir(&paths.cellar)
            .map(|d| d.count())
            .unwrap_or(0);
        println!("{} ({} packages)", style("✓").green(), count);
    } else {
        println!("{}", style("not found").yellow());
    }

    // Check installed state
    print!("  Checking installed packages state... ");
    match InstalledPackages::load(&paths) {
        Ok(installed) => {
            println!("{} ({} tracked)", style("✓").green(), installed.count());
        }
        Err(e) => {
            println!("{}", style("✗").red());
            println!("    {}", style(format!("Error: {}", e)).red());
            issues += 1;
        }
    }

    // Scan Cellar once for both drift and placeholder checks
    let cellar_packages = if paths.cellar.exists() {
        scan_cellar(&paths.cellar).ok()
    } else {
        None
    };

    // Check for Homebrew drift (Cellar + Caskroom)
    print!("  Checking for Homebrew drift... ");
    std::io::stdout().flush().ok();
    if let Some(ref cellar_packages) = cellar_packages {
        match InstalledPackages::load(&paths) {
            Ok(installed) => {
                let cellar_names: std::collections::HashSet<&str> =
                    cellar_packages.iter().map(|p| p.name.as_str()).collect();

                let mut added = 0usize;
                let mut removed = 0usize;
                let mut changed = 0usize;

                for pkg in cellar_packages {
                    match installed.get(&pkg.name) {
                        None => added += 1,
                        Some(state_pkg) if state_pkg.version != pkg.version => changed += 1,
                        _ => {}
                    }
                }

                for (name, _) in installed.iter() {
                    if !cellar_names.contains(name.as_str()) {
                        removed += 1;
                    }
                }

                let total_drift = added + removed + changed;
                if total_drift == 0 {
                    println!("{}", style("✓").green());
                } else if args.fix {
                    println!();
                    match crate::cli::sync::fix_drift(&paths).await {
                        Ok(descriptions) if !descriptions.is_empty() => {
                            for desc in &descriptions {
                                println!("    {} {}", style("✓").green(), desc);
                            }
                        }
                        Ok(_) => {
                            println!("    {} no changes needed", style("✓").green());
                        }
                        Err(e) => {
                            println!("    {} sync failed: {}", style("✗").red(), e);
                            issues += 1;
                        }
                    }
                } else {
                    println!("{}", style(format!("{} drifted", total_drift)).yellow());
                    if added > 0 {
                        println!(
                            "    {} {} in Homebrew but not tracked",
                            style(format!("{}", added)).yellow(),
                            if added == 1 { "package" } else { "packages" }
                        );
                    }
                    if removed > 0 {
                        println!(
                            "    {} {} tracked but not in Homebrew",
                            style(format!("{}", removed)).yellow(),
                            if removed == 1 { "package" } else { "packages" }
                        );
                    }
                    if changed > 0 {
                        println!(
                            "    {} {} with version mismatch",
                            style(format!("{}", changed)).yellow(),
                            if changed == 1 { "package" } else { "packages" }
                        );
                    }
                    println!(
                        "    {}",
                        style("Run 'stout sync' or 'stout doctor --fix' to reconcile").dim()
                    );
                    issues += 1;
                }
            }
            _ => {
                println!("{}", style("skipped").dim());
            }
        }
    } else {
        println!("{}", style("skipped (no Cellar)").dim());
    }

    // Check patchelf on Linux (required for ELF binary relocation)
    #[cfg(target_os = "linux")]
    {
        print!("  Checking patchelf (ELF relocator)... ");
        if std::process::Command::new("patchelf")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            println!("{}", style("✓").green());
        } else {
            println!("{}", style("✗ not found").red());
            println!(
                "    {}",
                style("patchelf is required for Homebrew bottles to work on Linux").yellow()
            );
            println!(
                "    {}",
                style("Install with: sudo apt install patchelf").dim()
            );
            issues += 1;
        }
    }

    // Check for unresolved placeholders and invalid signatures
    print!("  Checking for unresolved placeholders... ");
    std::io::stdout().flush().ok();
    if cellar_packages.is_none() {
        println!("{}", style("skipped (no Cellar)").dim());
    } else {
        println!();
        // Use fix_upgrade_issues with should_fix flag to control behavior
        match fix_upgrade_issues(vec![], &paths, args.fix).await {
            Ok(results) => {
                if args.fix {
                    // Report what was fixed
                    if results.placeholders_fixed > 0 {
                        println!(
                            "    {} Relocated {} placeholder files",
                            style("✓").green(),
                            results.placeholders_fixed
                        );
                    }
                    if results.signatures_fixed > 0 {
                        println!(
                            "    {} Re-signed {} files",
                            style("✓").green(),
                            results.signatures_fixed
                        );
                    }
                    if !results.packages_reinstalled.is_empty() {
                        println!(
                            "    {} Reinstalled {} corrupted package(s)",
                            style("✓").green(),
                            results.packages_reinstalled.len()
                        );
                    }
                    if results.placeholders_fixed == 0
                        && results.signatures_fixed == 0
                        && results.packages_reinstalled.is_empty()
                    {
                        println!("    {} No issues found", style("✓").green());
                    }
                } else {
                    // Report what needs fixing
                    if results.placeholders_fixed == 0 {
                        println!(
                            "    {} No unresolved placeholders found",
                            style("✓").green()
                        );
                    } else {
                        println!(
                            "    {} {} unresolved placeholders",
                            style("⚠").yellow(),
                            results.placeholders_fixed
                        );
                        issues += 1;
                    }

                    // Report signature issues (macOS only)
                    #[cfg(target_os = "macos")]
                    {
                        if results.packages_with_broken_sigs.is_empty() {
                            println!("    {} No invalid signatures found", style("✓").green());
                        } else {
                            let total_sigs: usize = results
                                .packages_with_broken_sigs
                                .iter()
                                .map(|(_, c)| c)
                                .sum();
                            println!(
                                "    {} {} files with invalid signatures",
                                style("⚠").yellow(),
                                total_sigs
                            );
                            for (name, count) in &results.packages_with_broken_sigs {
                                println!("      {} {} ({} files)", style("•").dim(), name, count);
                            }
                            issues += 1;
                        }
                    }

                    #[cfg(target_os = "macos")]
                    let has_broken_sigs = !results.packages_with_broken_sigs.is_empty();
                    #[cfg(not(target_os = "macos"))]
                    let has_broken_sigs = false;

                    if results.placeholders_fixed > 0 || has_broken_sigs {
                        println!(
                            "    {}",
                            style("Run 'stout doctor --fix' to fix all issues").dim()
                        );
                    }
                }
            }
            Err(e) => {
                println!("    {} Failed to check issues: {}", style("✗").red(), e);
                issues += 1;
            }
        }
    }

    // Code signatures check is now integrated above (macOS only)

    // Check dynamic library dependencies (macOS only)
    #[cfg(target_os = "macos")]
    {
        use rayon::prelude::*;

        print!("  Checking dynamic library dependencies... ");
        std::io::stdout().flush().ok();

        if let Some(ref cellar_packages) = cellar_packages {
            // (pkg_name, [unique missing dylib paths])
            let affected: Vec<(String, Vec<String>)> = cellar_packages
                .par_iter()
                .filter_map(|pkg| {
                    let mut seen = std::collections::HashSet::new();
                    for file in walk_macho_files(&pkg.path) {
                        for dylib in missing_dylibs(&file, &paths.prefix) {
                            seen.insert(dylib);
                        }
                    }
                    if seen.is_empty() {
                        None
                    } else {
                        let mut missing: Vec<String> = seen.into_iter().collect();
                        missing.sort();
                        Some((pkg.name.clone(), missing))
                    }
                })
                .collect();

            if affected.is_empty() {
                println!("{}", style("✓").green());
            } else if args.fix {
                println!();
                // Derive unique missing dependency package names from the opt/Cellar paths
                let mut seen_pkgs = std::collections::HashSet::new();
                let mut missing_pkgs: Vec<String> = Vec::new();
                for (_, dylibs) in &affected {
                    for dylib in dylibs {
                        if let Some(pkg) = package_from_dylib_path(dylib, &paths.prefix) {
                            if seen_pkgs.insert(pkg.clone()) {
                                missing_pkgs.push(pkg);
                            }
                        }
                    }
                }
                if missing_pkgs.is_empty() {
                    println!(
                        "    {} could not determine packages to install",
                        style("✗").red()
                    );
                    issues += 1;
                } else {
                    println!(
                        "    {} Installing {} missing {}...",
                        style("→").cyan(),
                        missing_pkgs.len(),
                        if missing_pkgs.len() == 1 {
                            "dependency"
                        } else {
                            "dependencies"
                        }
                    );
                    for pkg in &missing_pkgs {
                        println!("      {} {}", style("•").dim(), pkg);
                    }
                    let install_args = crate::cli::install::Args {
                        formulas: missing_pkgs,
                        ignore_dependencies: false,
                        dry_run: false,
                        build_from_source: false,
                        head: false,
                        keep_bottles: false,
                        jobs: None,
                        cc: None,
                        cxx: None,
                        force: false,
                        cask: false,
                        formula: false,
                        no_verify: false,
                        appdir: None,
                    };
                    if let Err(e) = crate::cli::install::run(install_args).await {
                        println!("    {} Install failed: {}", style("✗").red(), e);
                        issues += 1;
                    }
                }
            } else {
                let total_missing: usize = affected.iter().map(|(_, m)| m.len()).sum();
                println!(
                    "{}",
                    style(format!(
                        "{} missing dylib(s) across {} package(s)",
                        total_missing,
                        affected.len()
                    ))
                    .yellow()
                );
                for (name, missing) in &affected {
                    println!(
                        "    {} {} ({} missing dylib(s))",
                        style("⚠").yellow(),
                        name,
                        missing.len()
                    );
                    for dylib in missing {
                        println!("      {} {}", style("•").dim(), dylib);
                    }
                }
                println!(
                    "    {}",
                    style("Run 'stout doctor --fix' to install missing dependencies, or 'stout upgrade <package>' if the dependency soname changed").dim()
                );
                issues += 1;
            }
        } else {
            println!("{}", style("skipped (no Cellar)").dim());
        }
    }

    // Summary
    println!();
    if issues == 0 {
        println!("{}", style("Your system is ready to brew!").green().bold());
    } else {
        println!(
            "{}",
            style(format!("Found {} issue(s)", issues)).yellow().bold()
        );
    }
    println!();

    Ok(())
}

#[cfg(target_os = "macos")]
fn is_macho_file(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 4];
    if std::io::Read::read_exact(&mut file, &mut buf).is_err() {
        return false;
    }
    let magic = u32::from_be_bytes(buf);
    // Mach-O magic numbers (both endianness covered by BE read):
    // 0xFEEDFACE - 32-bit
    // 0xFEEDFACF - 64-bit
    // 0xCEFAEDFE - 32-bit little-endian (shows as 0xCEFAEDFE in BE)
    // 0xCFFAEDFE - 64-bit little-endian (shows as 0xCFFAEDFE in BE)
    matches!(magic, 0xFEEDFACE | 0xFEEDFACF | 0xCEFAEDFE | 0xCFFAEDFE)
}

#[cfg(target_os = "macos")]
fn verify_codesign(path: &Path) -> bool {
    let Ok(output) = std::process::Command::new("codesign")
        .arg("-v")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
    else {
        return true; // can't check, assume OK
    };

    if output.status.success() {
        return true;
    }

    // "not signed at all" is normal for object files, scripts, etc.
    // Only flag if the binary HAS a signature that is actually invalid.
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("code object is not signed at all") {
        return true; // unsigned is fine
    }
    false // signature present but invalid
}

#[cfg(target_os = "macos")]
fn missing_dylibs(path: &Path, prefix: &std::path::Path) -> Vec<String> {
    let output = match std::process::Command::new("otool")
        .arg("-L")
        .arg(path)
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let prefix_str = prefix.to_string_lossy();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen = std::collections::HashSet::new();
    let mut missing = Vec::new();

    for line in stdout.lines().skip(1) {
        let line = line.trim();
        // Lines look like: "/path/to/lib.dylib (compatibility version X, current version Y)"
        // Skip @rpath/@loader_path/@executable_path — relative, not resolvable here
        if line.starts_with('@') {
            continue;
        }
        let dylib_path = line.find(" (").map_or(line, |idx| &line[..idx]);
        // Only check paths under the Homebrew prefix (skip system dylibs)
        if !dylib_path.starts_with(prefix_str.as_ref()) {
            continue;
        }
        if !std::path::Path::new(dylib_path).exists()
            && !is_python_ext_false_positive(dylib_path)
            && seen.insert(dylib_path.to_string())
        {
            missing.push(dylib_path.to_string());
        }
    }

    missing
}

/// Python C extensions embed the dotted module name in their install name, e.g.
/// `cryptography.hazmat.bindings._rust.abi3.so` when the actual file on disk is
/// `_rust.abi3.so` in the same directory.  Detect this by looking for `._` in the
/// filename and checking whether the suffix (the real filename) exists.
#[cfg(target_os = "macos")]
fn is_python_ext_false_positive(dylib_path: &str) -> bool {
    let path = std::path::Path::new(dylib_path);
    let parent = match path.parent() {
        Some(p) if p.exists() => p,
        _ => return false,
    };
    let filename = match path.file_name().and_then(|n| n.to_str()) {
        Some(f) => f,
        None => return false,
    };
    // e.g. "cryptography.hazmat.bindings._rust.abi3.so" → look for "._" marker
    if let Some(pos) = filename.find("._") {
        let real_name = &filename[pos + 1..]; // "_rust.abi3.so"
        return parent.join(real_name).exists();
    }
    false
}

/// Extract the Homebrew formula name from a dylib path under the prefix.
///
/// `/opt/homebrew/opt/capstone/lib/libcapstone.5.dylib`  → `capstone`
/// `/opt/homebrew/Cellar/simdjson/4.6.1/lib/…`           → `simdjson`
#[cfg(target_os = "macos")]
fn package_from_dylib_path(dylib_path: &str, prefix: &std::path::Path) -> Option<String> {
    let prefix_str = prefix.to_string_lossy();
    for subdir in &["opt", "Cellar"] {
        let needle = format!("{}/{}/", prefix_str, subdir);
        if let Some(rest) = dylib_path.strip_prefix(needle.as_str()) {
            return rest.split('/').next().map(|s| s.to_string());
        }
    }
    None
}

#[cfg(target_os = "macos")]
fn resign_file(path: &Path) -> bool {
    std::process::Command::new("codesign")
        .arg("--force")
        .arg("--sign")
        .arg("-")
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
