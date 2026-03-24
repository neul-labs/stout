//! First-run import prompt
//!
//! Detects when the Cellar has significant untracked packages
//! or when Homebrew has installed casks, and offers to import them.

use anyhow::Result;
use console::style;
use stout_cask::InstalledCasks;
use stout_install::cask_scan::scan_caskroom;
use stout_install::cellar::scan_cellar;
use stout_install::relocate_bottle;
use stout_state::{InstalledPackages, Paths};

use crate::cli::import::import_cellar_package;

/// Check if first-run/migration import should be offered.
///
/// Triggers when there are 5+ untracked packages/casks in the Cellar.
/// Respects the "already offered" marker (state file existence).
///
/// Returns `true` if packages were imported (caller should reload state).
/// Returns `false` if skipped or not applicable.
pub fn check_first_run_import(installed: &mut InstalledPackages, paths: &Paths) -> Result<bool> {
    // Only trigger when state file doesn't exist (file existence = user was offered and made a choice)
    if paths.installed_file().exists() {
        return Ok(false);
    }

    // Scan cellar and compute actual untracked packages (set difference)
    let cellar_packages = scan_cellar(&paths.cellar)?;
    let untracked_cellar: Vec<_> = cellar_packages
        .iter()
        .filter(|pkg| !installed.is_installed(&pkg.name))
        .collect();

    // Check for untracked casks
    let brew_casks = scan_caskroom(&paths.prefix).unwrap_or_default();
    let cask_state_path = paths.stout_dir.join("casks.json");
    let cask_state = InstalledCasks::load(&cask_state_path).unwrap_or_default();
    let untracked_casks: Vec<_> = brew_casks
        .iter()
        .filter(|cask| !cask_state.is_installed(&cask.token))
        .collect();

    // Only trigger if there are enough untracked items to be worth importing
    const MIN_UNTRACKED: usize = 5;
    let total_untracked = untracked_cellar.len() + untracked_casks.len();
    if total_untracked < MIN_UNTRACKED {
        return Ok(false);
    }

    // Check if we're in an interactive terminal
    if !crate::output::is_interactive() {
        return Ok(false);
    }

    // Build prompt message
    let state_count = installed.count();
    let mut items = Vec::new();
    if !untracked_cellar.is_empty() {
        items.push(format!("{} packages", untracked_cellar.len()));
    }
    if !untracked_casks.is_empty() {
        items.push(format!("{} casks", untracked_casks.len()));
    }
    let items_str = items.join(" and ");

    let msg = if state_count == 0 {
        format!(
            "{} {} in Homebrew but none tracked by stout.",
            style("Found").cyan(),
            items_str
        )
    } else {
        format!(
            "{} {} untracked in Homebrew (already tracking {} packages).",
            style("Found").cyan(),
            items_str,
            state_count
        )
    };

    println!("\n{}", msg);

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
    println!("\n{}...", style("Importing from Homebrew").cyan());

    // Import only untracked packages
    for pkg in &untracked_cellar {
        import_cellar_package(installed, pkg);
        // Relocate Homebrew placeholders (@@HOMEBREW_PREFIX@@, etc.)
        let _ = relocate_bottle(&pkg.path, &paths.prefix);
    }
    if !untracked_cellar.is_empty() {
        installed.save(paths)?;
    }

    // Import untracked casks
    if !untracked_casks.is_empty() {
        let mut cask_state = InstalledCasks::load(&cask_state_path).unwrap_or_default();
        for cask in &untracked_casks {
            let timestamp = crate::cli::import::timestamp_now_iso();
            let imported_cask = stout_cask::InstalledCask {
                version: cask.version.clone().unwrap_or_else(|| "unknown".to_string()),
                installed_at: timestamp,
                artifact_path: std::path::PathBuf::from(""),
                auto_updates: false,
                artifacts: Vec::new(),
            };
            cask_state.add(&cask.token, imported_cask);
        }
        cask_state.save(&cask_state_path)?;
    }

    println!(
        "  {} Imported {} items\n",
        style("✓").green(),
        total_untracked
    );

    Ok(true)
}

