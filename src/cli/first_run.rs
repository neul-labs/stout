//! First-run import prompt
//!
//! Detects when the Cellar has packages but Stout's state is empty,
//! and offers to import them.

use anyhow::Result;
use console::style;
use stout_install::cellar::{count_cellar_packages, scan_cellar};
use stout_state::{InstalledPackages, Paths};

use crate::cli::import::import_cellar_package;

/// Check if first-run import should be offered.
///
/// Returns `true` if packages were imported (caller should reload state).
/// Returns `false` if skipped or not applicable.
pub fn check_first_run_import(installed: &mut InstalledPackages, paths: &Paths) -> Result<bool> {
    // Only trigger when state is empty
    if installed.count() > 0 {
        return Ok(false);
    }

    // Only trigger when state file doesn't exist (empty file = user declined)
    if paths.installed_file().exists() {
        return Ok(false);
    }

    let cellar_count = count_cellar_packages(&paths.cellar);
    if cellar_count == 0 {
        return Ok(false);
    }

    // Check if we're in an interactive terminal
    if !atty_is_interactive() {
        return Ok(false);
    }

    println!(
        "\n{} {} packages in Homebrew Cellar but none tracked by stout.",
        style("Found").cyan(),
        cellar_count
    );

    // Prompt user
    eprint!("{} ", style("Import them? [Y/n]").bold());

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input == "n" || input == "no" {
        // Create empty state file so prompt doesn't repeat
        installed.save(paths)?;
        println!("{}", style("Skipped import.").dim());
        return Ok(false);
    }

    // Run import
    println!("\n{}...", style("Importing packages").cyan());

    let cellar_packages = scan_cellar(&paths.cellar)?;
    let mut count = 0u32;

    for pkg in &cellar_packages {
        import_cellar_package(installed, pkg);
        count += 1;
    }

    installed.save(paths)?;

    println!("  {} Imported {} packages\n", style("✓").green(), count);

    Ok(true)
}

/// Check if stdin is a TTY (interactive terminal).
fn atty_is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}
