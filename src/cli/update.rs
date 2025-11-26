//! Update command

use anyhow::{Context, Result};
use brewx_index::IndexSync;
use brewx_state::{Config, Paths};
use clap::Args as ClapArgs;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

#[derive(ClapArgs)]
pub struct Args {
    /// Force update even if index is fresh
    #[arg(short, long)]
    pub force: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    let sync = IndexSync::new(Some(&config.index.base_url), &paths.brewx_dir)?;

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
        manifest.formula_count
    );

    // Save manifest locally
    let manifest_json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(paths.manifest(), manifest_json)?;

    println!(
        "\n{}",
        style("Run 'brewx search <query>' to find packages").dim()
    );

    Ok(())
}
