//! Search command

use anyhow::{Context, Result};
use brewx_index::Database;
use brewx_state::Paths;
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Search query
    pub query: String,

    /// Maximum results to show
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    // Open the database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'brewx update' first.",
            style("error:").red().bold()
        );
        std::process::exit(1);
    }

    // Search
    let results = db.search(&args.query, args.limit)?;

    if results.is_empty() {
        println!(
            "\n{} No formulas found for '{}'",
            style("•").dim(),
            args.query
        );
        return Ok(());
    }

    println!(
        "\n{} {} formulas:\n",
        style("Found").cyan(),
        results.len()
    );

    // Find the longest name for alignment
    let max_name_len = results.iter().map(|f| f.name.len()).max().unwrap_or(0);
    let max_ver_len = results.iter().map(|f| f.version.len()).max().unwrap_or(0);

    for formula in &results {
        let desc = formula.desc.as_deref().unwrap_or("");
        let desc_truncated = if desc.len() > 50 {
            format!("{}...", &desc[..47])
        } else {
            desc.to_string()
        };

        println!(
            "  {:<width_name$}  {:<width_ver$}  {}",
            style(&formula.name).green(),
            style(&formula.version).dim(),
            style(desc_truncated).dim(),
            width_name = max_name_len,
            width_ver = max_ver_len,
        );
    }

    println!(
        "\n{}\n",
        style("Use 'brewx info <formula>' for details").dim()
    );

    Ok(())
}
