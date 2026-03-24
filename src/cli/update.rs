//! Update command

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use stout_index::IndexSync;
use stout_state::{Config, Paths};

#[derive(ClapArgs)]
pub struct Args {
    /// Force update even if index is fresh
    #[arg(short, long)]
    pub force: bool,

    /// Skip signature verification (development only)
    #[arg(long, hide = true)]
    pub insecure: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    // Use permissive security if --insecure flag is set (hidden, for dev only)
    let sync = if args.insecure {
        eprintln!(
            "{}",
            style("WARNING: Running without signature verification")
                .yellow()
                .bold()
        );
        IndexSync::permissive(Some(&config.index.base_url), &paths.stout_dir)?
    } else {
        IndexSync::with_security_policy(
            Some(&config.index.base_url),
            &paths.stout_dir,
            config.security.to_security_policy(),
        )?
    };

    // Show progress spinner
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    spinner.set_message("Fetching index...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    // Download index
    let manifest = sync
        .sync_index(paths.index_db())
        .await
        .context("Failed to sync index")?;

    spinner.finish_and_clear();

    // Show result
    println!(
        "\n{} to {} ({} formulas)",
        style("Updated").green().bold(),
        style(&manifest.version).cyan(),
        manifest.formula_count()
    );

    // Show signature info
    if let Some(signed_at) = manifest.signed_at {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_hours = (now.saturating_sub(signed_at)) / 3600;
        if manifest.signature.is_some() {
            println!(
                "{} {}",
                style("✓ Signature verified").green(),
                style(format!("(signed {}h ago)", age_hours)).dim()
            );
        }
    }

    // Save manifest locally
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(paths.manifest(), manifest_json)?;

    // Run Homebrew sync if configured
    if config.sync.sync_on_update {
        println!("\n{}...", style("Syncing with Homebrew").cyan());
        match super::sync::run_auto_sync(&paths).await {
            Ok(0) => {
                println!("  {}", style("State is in sync with Homebrew.").dim());
            }
            Ok(n) => {
                println!("  {} Synced {} changes", style("✓").green(), n);
            }
            Err(e) => {
                eprintln!("  {} Homebrew sync failed: {}", style("⚠").yellow(), e);
            }
        }
    }

    println!(
        "\n{}",
        style("Run 'stout search <query>' to find packages").dim()
    );

    Ok(())
}
