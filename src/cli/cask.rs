//! Cask command - install macOS/Linux applications

use anyhow::{bail, Context, Result};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::time::Instant;
use stout_cask::{install_cask, uninstall_cask, CaskInstallOptions, InstalledCasks};
use stout_index::{Database, IndexSync};
use stout_state::{Config, Paths};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: CaskCommand,
}

#[derive(Subcommand)]
pub enum CaskCommand {
    /// Install casks
    Install(InstallArgs),

    /// Uninstall casks
    Uninstall(UninstallArgs),

    /// Show cask information
    Info(InfoArgs),

    /// Search for casks
    Search(SearchArgs),

    /// List installed casks
    List(ListArgs),

    /// Show outdated casks
    Outdated(OutdatedArgs),

    /// Upgrade installed casks
    Upgrade(UpgradeArgs),
}

#[derive(ClapArgs)]
pub struct InstallArgs {
    /// Casks to install
    pub casks: Vec<String>,

    /// Force reinstall even if already installed
    #[arg(long, short)]
    pub force: bool,

    /// Skip checksum verification
    #[arg(long)]
    pub no_verify: bool,

    /// Custom application directory
    #[arg(long)]
    pub appdir: Option<String>,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(ClapArgs)]
pub struct UninstallArgs {
    /// Casks to uninstall
    pub casks: Vec<String>,

    /// Remove preferences and caches (zap)
    #[arg(long)]
    pub zap: bool,

    /// Force uninstall
    #[arg(long, short)]
    pub force: bool,
}

#[derive(ClapArgs)]
pub struct InfoArgs {
    /// Cask to show info for
    pub cask: String,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

#[derive(ClapArgs)]
pub struct SearchArgs {
    /// Search query
    pub query: String,

    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct ListArgs {
    /// Show versions
    #[arg(long, short)]
    pub versions: bool,

    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct OutdatedArgs {
    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct UpgradeArgs {
    /// Casks to upgrade (all if empty)
    pub casks: Vec<String>,

    /// Force upgrade
    #[arg(long, short)]
    pub force: bool,

    /// Show what would be done without doing it
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        CaskCommand::Install(args) => run_install(args).await,
        CaskCommand::Uninstall(args) => run_uninstall(args).await,
        CaskCommand::Info(args) => run_info(args).await,
        CaskCommand::Search(args) => run_search(args).await,
        CaskCommand::List(args) => run_list(args).await,
        CaskCommand::Outdated(args) => run_outdated(args).await,
        CaskCommand::Upgrade(args) => run_upgrade(args).await,
    }
}

async fn run_install(args: InstallArgs) -> Result<()> {
    let start = Instant::now();

    if args.casks.is_empty() {
        bail!("No casks specified");
    }

    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    // Open database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    if !db.is_initialized()? {
        eprintln!(
            "{} Index not initialized. Run 'stout update' first.",
            style("error:").red().bold()
        );
        std::process::exit(1);
    }

    // Get cask cache and state paths
    let cache_dir = paths.stout_dir.join("cache").join("casks");
    std::fs::create_dir_all(&cache_dir)?;
    let state_path = paths.stout_dir.join("casks.json");

    // Fetch cask data and install
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let options = CaskInstallOptions {
        force: args.force,
        no_verify: args.no_verify,
        appdir: args.appdir.map(|s| s.into()),
        dry_run: args.dry_run,
    };

    let mut installed_count = 0;
    for token in &args.casks {
        // Look up cask in database
        let cask = match db.get_cask(token)? {
            Some(c) => c,
            None => {
                // Try to find similar
                let suggestions = db.find_similar_casks(token, 3)?;
                eprintln!(
                    "\n{} cask '{}' not found",
                    style("error:").red().bold(),
                    token
                );
                if !suggestions.is_empty() {
                    eprintln!("\n{}:", style("Did you mean?").yellow());
                    for s in suggestions {
                        eprintln!("  {} {}", style("•").dim(), s);
                    }
                }
                continue;
            }
        };

        println!(
            "\n{} {} {}...",
            style("Installing").cyan(),
            token,
            style(&cask.version).dim()
        );

        // Fetch full cask data
        let full_cask = sync
            .fetch_cask_cached(token, None)
            .await
            .context(format!("Failed to fetch cask {}", token))?;

        // Install
        match install_cask(&full_cask, &cache_dir, &state_path, &options).await {
            Ok(path) => {
                println!(
                    "  {} {} installed to {}",
                    style("✓").green(),
                    token,
                    path.display()
                );
                installed_count += 1;
            }
            Err(e) => {
                eprintln!("  {} Failed to install {}: {}", style("✗").red(), token, e);
            }
        }
    }

    let elapsed = start.elapsed();
    if installed_count > 0 {
        println!(
            "\n{} {} cask(s) in {:.1}s",
            style("Installed").green().bold(),
            installed_count,
            elapsed.as_secs_f64()
        );
    }

    Ok(())
}

async fn run_uninstall(args: UninstallArgs) -> Result<()> {
    if args.casks.is_empty() {
        bail!("No casks specified");
    }

    let paths = Paths::default();
    let state_path = paths.stout_dir.join("casks.json");

    for token in &args.casks {
        println!("{} {}...", style("Uninstalling").cyan(), token);

        match uninstall_cask(token, &state_path, args.zap).await {
            Ok(_) => {
                println!("  {} {} uninstalled", style("✓").green(), token);
            }
            Err(e) => {
                eprintln!(
                    "  {} Failed to uninstall {}: {}",
                    style("✗").red(),
                    token,
                    e
                );
            }
        }
    }

    Ok(())
}

async fn run_info(args: InfoArgs) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    // Open database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    // Get cask from database
    let cask_info = db
        .get_cask(&args.cask)?
        .ok_or_else(|| anyhow::anyhow!("Cask '{}' not found", args.cask))?;

    // Fetch full cask data for more details
    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;
    let full_cask = sync
        .fetch_cask_cached(&args.cask, None)
        .await
        .context(format!("Failed to fetch cask {}", args.cask))?;

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&full_cask)?);
        return Ok(());
    }

    // Text output
    println!("{}: {}", style("Token").bold(), cask_info.token);
    println!("{}: {}", style("Version").bold(), cask_info.version);

    if let Some(desc) = &cask_info.desc {
        println!("{}: {}", style("Description").bold(), desc);
    }

    if let Some(homepage) = &cask_info.homepage {
        println!("{}: {}", style("Homepage").bold(), homepage);
    }

    if let Some(url) = full_cask.download_url() {
        println!("{}: {}", style("Download URL").bold(), url);
    }

    println!(
        "{}: {}",
        style("SHA256").bold(),
        full_cask.sha256.as_str().unwrap_or("no_check")
    );

    if full_cask.auto_updates {
        println!("{}: yes", style("Auto-updates").bold());
    }

    // Check if installed
    let state_path = paths.stout_dir.join("casks.json");
    let installed = InstalledCasks::load(&state_path)?;
    if let Some(inst) = installed.get(&args.cask) {
        println!(
            "\n{}: {} (at {})",
            style("Installed").green().bold(),
            inst.version,
            inst.artifact_path.display()
        );
    }

    Ok(())
}

async fn run_search(args: SearchArgs) -> Result<()> {
    let paths = Paths::default();

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    let results = db.search_casks(&args.query, 20)?;

    if results.is_empty() {
        println!("No casks found matching '{}'", args.query);
        return Ok(());
    }

    if args.json {
        let json_results: Vec<_> = results
            .iter()
            .map(|c| {
                serde_json::json!({
                    "token": c.token,
                    "version": c.version,
                    "desc": c.desc,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
        return Ok(());
    }

    println!("{} cask(s) found:\n", results.len());
    for cask in results {
        print!("{}", style(&cask.token).green().bold());
        print!(" {}", style(&cask.version).dim());
        if let Some(desc) = &cask.desc {
            print!(" - {}", desc);
        }
        println!();
    }

    Ok(())
}

async fn run_list(args: ListArgs) -> Result<()> {
    let paths = Paths::default();
    let state_path = paths.stout_dir.join("casks.json");

    let installed = InstalledCasks::load(&state_path)?;

    if installed.count() == 0 {
        println!("No casks installed");
        return Ok(());
    }

    if args.json {
        let json_list: Vec<_> = installed
            .iter()
            .map(|(token, cask)| {
                serde_json::json!({
                    "token": token,
                    "version": cask.version,
                    "installed_at": cask.installed_at,
                    "path": cask.artifact_path,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_list)?);
        return Ok(());
    }

    println!("{} cask(s) installed:\n", installed.count());
    for (token, cask) in installed.iter() {
        if args.versions {
            println!(
                "{} {}",
                style(token).green().bold(),
                style(&cask.version).dim()
            );
        } else {
            println!("{}", style(token).green().bold());
        }
    }

    Ok(())
}

async fn run_outdated(args: OutdatedArgs) -> Result<()> {
    let paths = Paths::default();
    let state_path = paths.stout_dir.join("casks.json");

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    let installed = InstalledCasks::load(&state_path)?;

    if installed.count() == 0 {
        println!("No casks installed");
        return Ok(());
    }

    let mut outdated = Vec::new();

    for (token, inst) in installed.iter() {
        if let Some(cask_info) = db.get_cask(token)? {
            if cask_info.version != inst.version {
                outdated.push((token.clone(), inst.version.clone(), cask_info.version));
            }
        }
    }

    if outdated.is_empty() {
        println!("All casks are up to date");
        return Ok(());
    }

    if args.json {
        let json_outdated: Vec<_> = outdated
            .iter()
            .map(|(token, current, latest)| {
                serde_json::json!({
                    "token": token,
                    "current_version": current,
                    "latest_version": latest,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_outdated)?);
        return Ok(());
    }

    println!("{} outdated cask(s):\n", outdated.len());
    for (token, current, latest) in outdated {
        println!(
            "{} {} -> {}",
            style(&token).yellow().bold(),
            style(&current).dim(),
            style(&latest).green()
        );
    }

    Ok(())
}

async fn run_upgrade(args: UpgradeArgs) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let cache_dir = paths.stout_dir.join("cache").join("casks");
    std::fs::create_dir_all(&cache_dir)?;

    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    let installed = InstalledCasks::load(&state_path)?;

    // Determine which casks to upgrade
    let casks_to_upgrade: Vec<String> = if args.casks.is_empty() {
        // Upgrade all outdated
        installed
            .iter()
            .filter_map(|(token, inst)| {
                db.get_cask(token)
                    .ok()
                    .flatten()
                    .filter(|c| c.version != inst.version)
                    .map(|_| token.clone())
            })
            .collect()
    } else {
        args.casks.clone()
    };

    if casks_to_upgrade.is_empty() {
        println!("Nothing to upgrade");
        return Ok(());
    }

    let sync = IndexSync::with_security_policy(
        Some(&config.index.base_url),
        &paths.stout_dir,
        config.security.to_security_policy(),
    )?;

    let options = CaskInstallOptions {
        force: true, // Force reinstall for upgrade
        no_verify: false,
        appdir: None,
        dry_run: args.dry_run,
    };

    for token in casks_to_upgrade {
        let full_cask = sync
            .fetch_cask_cached(&token, None)
            .await
            .context(format!("Failed to fetch cask {}", token))?;

        println!(
            "{} {} to {}...",
            style("Upgrading").cyan(),
            token,
            full_cask.version
        );

        match install_cask(&full_cask, &cache_dir, &state_path, &options).await {
            Ok(path) => {
                println!(
                    "  {} {} upgraded to {}",
                    style("✓").green(),
                    token,
                    path.display()
                );
            }
            Err(e) => {
                eprintln!("  {} Failed to upgrade {}: {}", style("✗").red(), token, e);
            }
        }
    }

    Ok(())
}
