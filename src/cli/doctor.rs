//! Doctor command - check system health

use anyhow::Result;
use stout_index::Database;
use stout_state::{Config, InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
    let paths = Paths::default();

    println!("\n{}", style("stout doctor").cyan().bold());
    println!("{}\n", style("Checking system health...").dim());

    let mut issues = 0;

    // Check stout directory
    print!("  Checking stout directory... ");
    if paths.stout_dir.exists() {
        println!("{}", style("✓").green());
    } else {
        println!("{} (will be created on first use)", style("missing").yellow());
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
                println!(
                    "    {}",
                    style("Run 'stout update' to initialize").dim()
                );
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
        println!(
            "{} ({} packages)",
            style("✓").green(),
            count
        );
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

    // Summary
    println!();
    if issues == 0 {
        println!(
            "{}",
            style("Your system is ready to brew!").green().bold()
        );
    } else {
        println!(
            "{}",
            style(format!("Found {} issue(s)", issues)).yellow().bold()
        );
    }
    println!();

    Ok(())
}
