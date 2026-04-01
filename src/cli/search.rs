//! Search command

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use console::style;
use stout_index::Database;
use stout_state::Paths;

#[derive(ClapArgs)]
pub struct Args {
    /// Search query
    pub query: String,

    /// Maximum results to show
    #[arg(short, long, default_value = "20")]
    pub limit: usize,

    /// Search only formulas
    #[arg(long)]
    pub formula: bool,

    /// Search only casks
    #[arg(long)]
    pub cask: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();

    // Open the database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'stout update' first.",
            style("error:").red().bold()
        );
        std::process::exit(1);
    }

    let search_formulas = !args.cask || args.formula;
    let search_casks = !args.formula || args.cask;

    let mut found_any = false;

    // Search formulas
    if search_formulas {
        let results = db.search(&args.query, args.limit)?;

        if !results.is_empty() {
            found_any = true;
            println!("\n{} {} formulas:\n", style("Found").cyan(), results.len());

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
        }
    }

    // Search casks
    if search_casks {
        let cask_results = db.search_casks(&args.query, args.limit)?;

        if !cask_results.is_empty() {
            found_any = true;
            println!(
                "\n{} {} casks:\n",
                style("Found").cyan(),
                cask_results.len()
            );

            let max_token_len = cask_results
                .iter()
                .map(|c| c.token.len())
                .max()
                .unwrap_or(0);
            let max_ver_len = cask_results
                .iter()
                .map(|c| c.version.len())
                .max()
                .unwrap_or(0);

            for cask in &cask_results {
                let desc = cask.desc.as_deref().unwrap_or("");
                let desc_truncated = if desc.len() > 50 {
                    format!("{}...", &desc[..47])
                } else {
                    desc.to_string()
                };

                println!(
                    "  {:<width_token$}  {:<width_ver$}  {}",
                    style(&cask.token).magenta(),
                    style(&cask.version).dim(),
                    style(desc_truncated).dim(),
                    width_token = max_token_len,
                    width_ver = max_ver_len,
                );
            }
        }
    }

    if !found_any {
        println!(
            "\n{} No results found for '{}'",
            style("•").dim(),
            args.query
        );
    } else {
        println!("\n{}\n", style("Use 'stout info <name>' for details").dim());
    }

    Ok(())
}
