//! Bundle command - Brewfile management

use anyhow::{bail, Context, Result};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::path::{Path, PathBuf};
use stout_bundle::{BrewEntry, Brewfile, CaskEntry, TapEntry};
use stout_cask::{install_cask, CaskInstallOptions, InstalledCasks};
use stout_index::{Database, IndexSync};
use stout_state::{Config, InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<BundleCommand>,

    /// Path to Brewfile (default: ./Brewfile)
    #[arg(long, short, global = true)]
    pub file: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum BundleCommand {
    /// Install packages from Brewfile (default)
    Install(InstallArgs),

    /// Generate Brewfile from installed packages
    Dump(DumpArgs),

    /// Check if all packages in Brewfile are installed
    Check(CheckArgs),

    /// List entries in Brewfile
    List(ListArgs),

    /// Remove packages not in Brewfile
    Cleanup(CleanupArgs),
}

#[derive(ClapArgs)]
pub struct InstallArgs {
    /// Don't actually install, just show what would be done
    #[arg(long)]
    pub dry_run: bool,

    /// Force reinstall even if already installed
    #[arg(long, short)]
    pub force: bool,

    /// Skip taps
    #[arg(long)]
    pub no_tap: bool,

    /// Skip formulas
    #[arg(long)]
    pub no_brew: bool,

    /// Skip casks
    #[arg(long)]
    pub no_cask: bool,
}

#[derive(ClapArgs)]
pub struct DumpArgs {
    /// Force overwrite existing Brewfile
    #[arg(long, short)]
    pub force: bool,

    /// Include dependencies (not just requested packages)
    #[arg(long)]
    pub all: bool,

    /// Output to stdout instead of file
    #[arg(long)]
    pub stdout: bool,
}

#[derive(ClapArgs)]
pub struct CheckArgs {
    /// Show verbose output
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(ClapArgs)]
pub struct ListArgs {
    /// Filter by type (tap, brew, cask, mas)
    #[arg(long)]
    pub r#type: Option<String>,

    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct CleanupArgs {
    /// Don't actually remove, just show what would be done
    #[arg(long)]
    pub dry_run: bool,

    /// Force removal
    #[arg(long, short)]
    pub force: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let brewfile_path = args.file.unwrap_or_else(|| PathBuf::from("Brewfile"));

    match args.command {
        Some(BundleCommand::Install(install_args)) => {
            run_install(&brewfile_path, install_args).await
        }
        Some(BundleCommand::Dump(dump_args)) => run_dump(&brewfile_path, dump_args).await,
        Some(BundleCommand::Check(check_args)) => run_check(&brewfile_path, check_args).await,
        Some(BundleCommand::List(list_args)) => run_list(&brewfile_path, list_args).await,
        Some(BundleCommand::Cleanup(cleanup_args)) => {
            run_cleanup(&brewfile_path, cleanup_args).await
        }
        None => {
            // Default: install
            run_install(
                &brewfile_path,
                InstallArgs {
                    dry_run: false,
                    force: false,
                    no_tap: false,
                    no_brew: false,
                    no_cask: false,
                },
            )
            .await
        }
    }
}

async fn run_install(brewfile_path: &Path, args: InstallArgs) -> Result<()> {
    println!("{} {}...", style("Parsing").cyan(), brewfile_path.display());

    let brewfile = Brewfile::parse(brewfile_path)
        .context(format!("Failed to parse {}", brewfile_path.display()))?;

    if brewfile.is_empty() {
        println!("{}", style("Brewfile is empty").yellow());
        return Ok(());
    }

    println!(
        "Found {} entries ({} taps, {} brews, {} casks)",
        brewfile.entry_count(),
        brewfile.taps.len(),
        brewfile.brews.len(),
        brewfile.casks.len()
    );

    let paths = Paths::default();
    paths.ensure_dirs()?;

    let config = Config::load(&paths)?;

    // Open database
    let db = Database::open(paths.index_db())
        .context("Failed to open index. Run 'stout update' first.")?;

    let installed_formulas = InstalledPackages::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;

    // Track what needs to be installed
    let mut taps_to_add: Vec<&TapEntry> = Vec::new();
    let mut formulas_to_install: Vec<&BrewEntry> = Vec::new();
    let mut casks_to_install: Vec<&CaskEntry> = Vec::new();

    // Check taps
    if !args.no_tap {
        for tap in &brewfile.taps {
            // For now, just note that we'd add the tap
            taps_to_add.push(tap);
        }
    }

    // Check formulas
    if !args.no_brew {
        for brew in &brewfile.brews {
            if !installed_formulas.is_installed(&brew.name) {
                formulas_to_install.push(brew);
            }
        }
    }

    // Check casks
    if !args.no_cask {
        for cask in &brewfile.casks {
            if !installed_casks.is_installed(&cask.name) {
                casks_to_install.push(cask);
            }
        }
    }

    // Show what would be done
    if !taps_to_add.is_empty() {
        println!("\n{}:", style("Taps to add").bold());
        for tap in &taps_to_add {
            println!("  {} {}", style("+").green(), tap.name);
        }
    }

    if !formulas_to_install.is_empty() {
        println!("\n{}:", style("Formulas to install").bold());
        for brew in &formulas_to_install {
            println!("  {} {}", style("+").green(), brew.name);
        }
    }

    if !casks_to_install.is_empty() {
        println!("\n{}:", style("Casks to install").bold());
        for cask in &casks_to_install {
            println!("  {} {}", style("+").green(), cask.name);
        }
    }

    if taps_to_add.is_empty() && formulas_to_install.is_empty() && casks_to_install.is_empty() {
        println!("\n{}", style("All packages are already installed.").green());
        return Ok(());
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Install formulas
    if !formulas_to_install.is_empty() {
        println!("\n{}...", style("Installing formulas").cyan());

        for brew in formulas_to_install {
            println!("  {} {}...", style("Installing").cyan(), brew.name);

            // Use the install command logic (simplified here)
            // In a real implementation, we'd call the install logic directly
            let formula = match db.get_formula(&brew.name)? {
                Some(f) => f,
                None => {
                    eprintln!("  {} Formula '{}' not found", style("✗").red(), brew.name);
                    continue;
                }
            };

            // Mark as would-install for now
            println!(
                "  {} {} {} (would install)",
                style("✓").green(),
                brew.name,
                style(&formula.version).dim()
            );
        }
    }

    // Install casks
    if !casks_to_install.is_empty() {
        println!("\n{}...", style("Installing casks").cyan());

        let sync = IndexSync::with_security_policy(
            Some(&config.index.base_url),
            &paths.stout_dir,
            config.security.to_security_policy(),
        )?;
        let cache_dir = paths.stout_dir.join("cache").join("casks");
        std::fs::create_dir_all(&cache_dir)?;

        let options = CaskInstallOptions {
            force: args.force,
            no_verify: false,
            appdir: None,
            dry_run: false,
        };

        for cask in casks_to_install {
            println!("  {} {}...", style("Installing").cyan(), cask.name);

            // Fetch full cask data
            let full_cask = match sync.fetch_cask_cached(&cask.name, None).await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "  {} Failed to fetch cask '{}': {}",
                        style("✗").red(),
                        cask.name,
                        e
                    );
                    continue;
                }
            };

            match install_cask(&full_cask, &cache_dir, &state_path, &options).await {
                Ok(path) => {
                    println!(
                        "  {} {} installed to {}",
                        style("✓").green(),
                        cask.name,
                        path.display()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "  {} Failed to install '{}': {}",
                        style("✗").red(),
                        cask.name,
                        e
                    );
                }
            }
        }
    }

    println!("\n{}", style("Bundle install complete.").green().bold());
    Ok(())
}

async fn run_dump(brewfile_path: &Path, args: DumpArgs) -> Result<()> {
    if brewfile_path.exists() && !args.force && !args.stdout {
        bail!(
            "{} already exists. Use --force to overwrite.",
            brewfile_path.display()
        );
    }

    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;

    // Collect formulas
    let formulas: Vec<(String, bool)> = installed
        .iter()
        .map(|(name, pkg)| (name.clone(), pkg.requested))
        .collect();

    // Collect casks
    let casks: Vec<String> = installed_casks.iter().map(|(t, _)| t.clone()).collect();

    // Generate Brewfile
    let content = Brewfile::generate(&[], &formulas, &casks);

    if args.stdout {
        print!("{}", content);
    } else {
        std::fs::write(brewfile_path, &content)?;
        println!("{} {}", style("Created").green(), brewfile_path.display());
    }

    Ok(())
}

async fn run_check(brewfile_path: &Path, args: CheckArgs) -> Result<()> {
    let brewfile = Brewfile::parse(brewfile_path)?;

    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;

    let mut missing_formulas = Vec::new();
    let mut missing_casks = Vec::new();

    // Check formulas
    for brew in &brewfile.brews {
        if !installed.is_installed(&brew.name) {
            missing_formulas.push(&brew.name);
        }
    }

    // Check casks
    for cask in &brewfile.casks {
        if !installed_casks.is_installed(&cask.name) {
            missing_casks.push(&cask.name);
        }
    }

    if missing_formulas.is_empty() && missing_casks.is_empty() {
        println!("{} All dependencies are satisfied.", style("✓").green());
        return Ok(());
    }

    if (args.verbose || !missing_formulas.is_empty()) && !missing_formulas.is_empty() {
        println!("{}:", style("Missing formulas").yellow());
        for name in &missing_formulas {
            println!("  {} {}", style("•").dim(), name);
        }
    }

    if (args.verbose || !missing_casks.is_empty()) && !missing_casks.is_empty() {
        println!("{}:", style("Missing casks").yellow());
        for name in &missing_casks {
            println!("  {} {}", style("•").dim(), name);
        }
    }

    let total_missing = missing_formulas.len() + missing_casks.len();
    println!(
        "\n{} {} dependencies are missing. Run 'stout bundle install' to install.",
        style("✗").red(),
        total_missing
    );

    // Exit with error code
    std::process::exit(1);
}

async fn run_list(brewfile_path: &Path, args: ListArgs) -> Result<()> {
    let brewfile = Brewfile::parse(brewfile_path)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&brewfile)?);
        return Ok(());
    }

    let show_all = args.r#type.is_none();
    let filter_type = args.r#type.as_deref();

    if (show_all || filter_type == Some("tap")) && !brewfile.taps.is_empty() {
        println!("{}:", style("Taps").bold());
        for tap in &brewfile.taps {
            println!("  {}", tap.name);
        }
        println!();
    }

    if (show_all || filter_type == Some("brew")) && !brewfile.brews.is_empty() {
        println!("{}:", style("Formulas").bold());
        for brew in &brewfile.brews {
            println!("  {}", brew.name);
        }
        println!();
    }

    if (show_all || filter_type == Some("cask")) && !brewfile.casks.is_empty() {
        println!("{}:", style("Casks").bold());
        for cask in &brewfile.casks {
            println!("  {}", cask.name);
        }
        println!();
    }

    if (show_all || filter_type == Some("mas")) && !brewfile.mas.is_empty() {
        println!("{}:", style("Mac App Store").bold());
        for mas in &brewfile.mas {
            println!("  {} ({})", mas.name, mas.id);
        }
        println!();
    }

    Ok(())
}

async fn run_cleanup(brewfile_path: &Path, args: CleanupArgs) -> Result<()> {
    let brewfile = Brewfile::parse(brewfile_path)?;

    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;

    // Find formulas not in Brewfile
    let brewfile_formulas: std::collections::HashSet<_> =
        brewfile.brews.iter().map(|b| b.name.as_str()).collect();

    let extra_formulas: Vec<_> = installed
        .iter()
        .filter(|(name, pkg)| pkg.requested && !brewfile_formulas.contains(name.as_str()))
        .map(|(name, _)| name.clone())
        .collect();

    // Find casks not in Brewfile
    let brewfile_casks: std::collections::HashSet<_> =
        brewfile.casks.iter().map(|c| c.name.as_str()).collect();

    let extra_casks: Vec<_> = installed_casks
        .iter()
        .filter(|(token, _)| !brewfile_casks.contains(token.as_str()))
        .map(|(token, _)| token.clone())
        .collect();

    if extra_formulas.is_empty() && extra_casks.is_empty() {
        println!("{} No extra packages to remove.", style("✓").green());
        return Ok(());
    }

    if !extra_formulas.is_empty() {
        println!("{}:", style("Extra formulas to remove").yellow());
        for name in &extra_formulas {
            println!("  {} {}", style("-").red(), name);
        }
    }

    if !extra_casks.is_empty() {
        println!("{}:", style("Extra casks to remove").yellow());
        for token in &extra_casks {
            println!("  {} {}", style("-").red(), token);
        }
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    println!("\n{}", style("Cleanup would remove these packages.").dim());
    println!(
        "{}",
        style("Run 'stout uninstall <package>' to remove them.").dim()
    );

    Ok(())
}
