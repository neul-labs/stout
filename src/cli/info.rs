//! Info command

use anyhow::{Context, Result};
use brewx_index::{Database, IndexSync};
use brewx_state::{Config, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula name
    pub formula: String,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    // Open the database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'brewx update' first.")?;

    // Get basic info from index
    let info = match db.get_formula(&args.formula)? {
        Some(info) => info,
        None => {
            // Try to find suggestions
            let suggestions = db.find_similar(&args.formula, 3)?;

            eprintln!(
                "\n{} formula '{}' not found",
                style("error:").red().bold(),
                args.formula
            );

            if !suggestions.is_empty() {
                eprintln!("\n{}:", style("Did you mean?").yellow());
                for s in suggestions {
                    eprintln!("  {} {}", style("•").dim(), s);
                }
            }

            eprintln!(
                "\n{}",
                style("Run 'brewx search <query>' to find packages").dim()
            );
            std::process::exit(1);
        }
    };

    // Fetch full formula data
    let sync = IndexSync::new(Some(&config.index.base_url), &paths.brewx_dir)?;
    let formula = sync
        .fetch_formula_cached(&args.formula, info.json_hash.as_deref())
        .await
        .context("Failed to fetch formula details")?;

    // Display
    println!();
    println!(
        "{} {}",
        style(&formula.name).green().bold(),
        style(&formula.version).cyan()
    );

    if let Some(desc) = &formula.desc {
        println!("{}", style(desc).dim());
    }

    println!();

    // Metadata
    if let Some(homepage) = &formula.homepage {
        println!("{:12} {}", style("Homepage:").dim(), homepage);
    }
    if let Some(license) = &formula.license {
        println!("{:12} {}", style("License:").dim(), license);
    }
    println!("{:12} {}", style("Tap:").dim(), formula.tap);

    // Dependencies
    if !formula.dependencies.runtime.is_empty()
        || !formula.dependencies.build.is_empty()
    {
        println!("\n{}:", style("Dependencies").cyan());

        let deps = &formula.dependencies;
        let total_deps = deps.runtime.len() + deps.build.len();
        let mut shown = 0;

        for (i, dep) in deps.runtime.iter().enumerate() {
            let is_last = i == deps.runtime.len() - 1 && deps.build.is_empty();
            let prefix = if is_last { "└──" } else { "├──" };
            println!("  {} {} {}", prefix, dep, style("(runtime)").dim());
            shown += 1;
        }

        for (i, dep) in deps.build.iter().enumerate() {
            let is_last = i == deps.build.len() - 1;
            let prefix = if is_last { "└──" } else { "├──" };
            println!("  {} {} {}", prefix, dep, style("(build)").dim());
        }
    }

    // Bottles
    if !formula.bottles.is_empty() {
        println!("\n{}:", style("Bottles").cyan());
        let platforms: Vec<_> = formula.bottles.keys().collect();
        let mut line = String::from("  ");
        for platform in platforms {
            line.push_str(&format!("{} {}  ", style("✓").green(), platform));
        }
        println!("{}", line);
    }

    // Caveats
    if let Some(caveats) = &formula.caveats {
        println!("\n{}:", style("Caveats").yellow());
        for line in caveats.lines() {
            println!("  {}", line);
        }
    }

    // Install status
    println!();
    let installed = paths.is_installed(&formula.name, &formula.version);
    if installed {
        println!(
            "{}: {} {}",
            style("Installed").green(),
            formula.name,
            formula.version
        );
    } else {
        println!("{}: {}", style("Installed").dim(), style("No").dim());
    }

    println!();
    Ok(())
}
