//! Test command - test installed formulas
//!
//! Runs basic smoke tests on installed packages by verifying binaries
//! can execute with --version or --help flags.

use anyhow::{bail, Context, Result};
use stout_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;
use std::process::Command;
use std::time::Instant;

#[derive(ClapArgs)]
pub struct Args {
    /// Formulas to test
    pub formulas: Vec<String>,

    /// Show verbose output
    #[arg(long, short)]
    pub verbose: bool,

    /// Test all installed packages
    #[arg(long)]
    pub all: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let start = Instant::now();

    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    // Get list of packages to test
    let packages: Vec<String> = if args.all {
        installed.names().cloned().collect()
    } else if args.formulas.is_empty() {
        bail!("No formulas specified. Use --all to test all installed packages.");
    } else {
        args.formulas.clone()
    };

    if packages.is_empty() {
        println!("{}", style("No packages to test.").dim());
        return Ok(());
    }

    println!(
        "\n{} {} packages...\n",
        style("Testing").cyan().bold(),
        packages.len()
    );

    let mut success_count = 0;
    let mut failure_count = 0;
    let mut skipped_count = 0;

    for name in &packages {
        // Check if formula is installed
        if !installed.is_installed(name) {
            println!(
                "  {} {} {}",
                style("○").dim(),
                name,
                style("(not installed)").dim()
            );
            skipped_count += 1;
            continue;
        }

        let pkg_info = installed.get(name)
            .with_context(|| format!("package '{}' is installed but not found in state", name))?;
        let install_path = paths
            .cellar
            .join(name)
            .join(&pkg_info.version);

        if !install_path.exists() {
            println!(
                "  {} {} {}",
                style("○").dim(),
                name,
                style("(installation not found)").dim()
            );
            skipped_count += 1;
            continue;
        }

        // Find binaries to test
        let bin_dir = install_path.join("bin");
        if !bin_dir.exists() {
            if args.verbose {
                println!(
                    "  {} {} {}",
                    style("○").dim(),
                    name,
                    style("(no binaries)").dim()
                );
            }
            skipped_count += 1;
            continue;
        }

        let mut tested = false;
        let mut all_passed = true;

        // Test each binary
        if let Ok(entries) = std::fs::read_dir(&bin_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let binary_name = path.file_name().unwrap().to_string_lossy().to_string();

                // Skip common non-executable files
                if binary_name.ends_with(".sh")
                    || binary_name.ends_with(".py")
                    || binary_name.ends_with(".rb")
                {
                    continue;
                }

                // Try running with --version
                let result = test_binary(&paths.prefix.join("bin").join(&binary_name), args.verbose);

                if result.is_ok() {
                    tested = true;
                    if args.verbose {
                        println!(
                            "    {} {}",
                            style("✓").green(),
                            binary_name
                        );
                    }
                } else {
                    tested = true;
                    all_passed = false;
                    if args.verbose {
                        println!(
                            "    {} {} - {}",
                            style("✗").red(),
                            binary_name,
                            result.unwrap_err()
                        );
                    }
                }
            }
        }

        if !tested {
            skipped_count += 1;
            if args.verbose {
                println!(
                    "  {} {} {}",
                    style("○").dim(),
                    name,
                    style("(no testable binaries)").dim()
                );
            }
        } else if all_passed {
            success_count += 1;
            println!(
                "  {} {} {}",
                style("✓").green(),
                name,
                style(&pkg_info.version).dim()
            );
        } else {
            failure_count += 1;
            println!(
                "  {} {} {}",
                style("✗").red(),
                name,
                style(&pkg_info.version).dim()
            );
        }
    }

    let elapsed = start.elapsed();

    println!();
    if failure_count == 0 {
        println!(
            "{} {} passed, {} skipped in {:.1}s",
            style("✓").green().bold(),
            success_count,
            skipped_count,
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "{} {} passed, {} failed, {} skipped in {:.1}s",
            style("!").yellow().bold(),
            success_count,
            failure_count,
            skipped_count,
            elapsed.as_secs_f64()
        );
    }

    if failure_count > 0 {
        std::process::exit(1);
    }

    Ok(())
}

fn test_binary(path: &std::path::Path, verbose: bool) -> Result<(), String> {
    // Check if the binary exists
    if !path.exists() {
        return Err("binary not found".to_string());
    }

    // Try --version first
    let version_result = Command::new(path)
        .arg("--version")
        .output();

    if let Ok(output) = version_result {
        if output.status.success() {
            if verbose {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if !stdout.is_empty() {
                    // Just show first line
                    if let Some(line) = stdout.lines().next() {
                        return Ok(());
                    }
                }
            }
            return Ok(());
        }
    }

    // Try --help as fallback
    let help_result = Command::new(path)
        .arg("--help")
        .output();

    if let Ok(output) = help_result {
        if output.status.success() || output.status.code() == Some(0) || output.status.code() == Some(1) {
            // Some programs exit with 1 for --help
            return Ok(());
        }
    }

    // Try just running it with no args (for simple utilities)
    let bare_result = Command::new(path)
        .output();

    if let Ok(output) = bare_result {
        // Consider it success if it runs without crashing
        if output.status.success() || output.status.code().is_some() {
            return Ok(());
        }
    }

    Err("failed to execute".to_string())
}
