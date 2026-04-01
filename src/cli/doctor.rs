//! Doctor command - check system health

use anyhow::Result;
use clap::Args as ClapArgs;
use console::style;
use std::io::Write;
use stout_index::Database;
use stout_install::cellar::scan_cellar;
use stout_install::{relocate_bottle, scan_cellar_unrelocated};
use stout_state::{Config, InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Automatically fix issues that can be repaired
    #[arg(long)]
    pub fix: bool,
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

    // Check for unresolved Homebrew placeholders
    print!("  Checking for unresolved placeholders... ");
    std::io::stdout().flush().ok();
    if let Some(ref cellar_packages) = cellar_packages {
        let affected_packages = scan_cellar_unrelocated(cellar_packages);

        let total_files: usize = affected_packages.iter().map(|(_, _, count)| count).sum();

        if affected_packages.is_empty() {
            println!("{}", style("✓").green());
        } else if args.fix {
            println!();
            let mut fixed = 0usize;
            for (name, path, _) in &affected_packages {
                match relocate_bottle(path, &paths.prefix) {
                    Ok(count) if count > 0 => {
                        fixed += count;
                        println!(
                            "    {} relocated {} files in {}",
                            style("✓").green(),
                            count,
                            name
                        );
                    }
                    Ok(_) => {}
                    Err(e) => {
                        println!("    {} {}: {}", style("✗").red(), name, e);
                    }
                }
            }
            if fixed > 0 {
                println!(
                    "    {} Fixed {} files across {} packages",
                    style("✓").green(),
                    fixed,
                    affected_packages.len()
                );
            }
        } else {
            println!(
                "{}",
                style(format!(
                    "{} files in {} packages",
                    total_files,
                    affected_packages.len()
                ))
                .yellow()
            );
            for (name, _, count) in &affected_packages {
                println!(
                    "    {} {} ({} files with @@HOMEBREW_*@@)",
                    style("⚠").yellow(),
                    name,
                    count
                );
            }
            println!(
                "    {}",
                style("Run 'stout doctor --fix' to relocate").dim()
            );
            issues += 1;
        }
    } else {
        println!("{}", style("skipped (no Cellar)").dim());
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
