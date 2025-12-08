//! Mirror command - create and serve offline mirrors

use anyhow::{bail, Result};
use stout_index::Database;
use stout_mirror::{
    create_mirror, detect_platform, serve_mirror, MirrorClient, MirrorClientConfig, MirrorConfig,
    MirrorManifest, ServeConfig,
};
use stout_state::Paths;
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use humansize::{format_size, BINARY};
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: MirrorCommand,
}

#[derive(Subcommand)]
pub enum MirrorCommand {
    /// Create a new mirror with specified packages
    Create {
        /// Output directory for the mirror
        output: PathBuf,

        /// Packages to include
        #[arg(required_unless_present = "all_installed")]
        packages: Vec<String>,

        /// Include all installed packages
        #[arg(long)]
        all_installed: bool,

        /// Create from Brewfile
        #[arg(long = "from-brewfile")]
        brewfile: Option<PathBuf>,

        /// Casks to include
        #[arg(long = "cask")]
        casks: Vec<String>,

        /// Linux apps to include
        #[arg(long = "linux-app")]
        linux_apps: Vec<String>,

        /// Platforms to include (default: current platform)
        #[arg(long)]
        platforms: Vec<String>,

        /// Include all platforms (warning: large download)
        #[arg(long)]
        all_platforms: bool,

        /// Skip dependency resolution
        #[arg(long)]
        no_deps: bool,

        /// Show what would be downloaded without actually downloading
        #[arg(long)]
        dry_run: bool,
    },

    /// Serve a mirror via HTTP
    Serve {
        /// Path to the mirror directory
        path: PathBuf,

        /// Port to listen on
        #[arg(long, short, default_value = "8080")]
        port: u16,

        /// Address to bind to
        #[arg(long, default_value = "0.0.0.0")]
        bind: String,

        /// Enable access logging
        #[arg(long)]
        log_access: bool,
    },

    /// Show information about a mirror
    Info {
        /// Path to the mirror directory
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check for outdated packages in mirror
    Outdated {
        /// Path to the mirror directory
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update packages in an existing mirror
    Update {
        /// Path to the mirror directory
        path: PathBuf,

        /// Specific packages to update (default: all)
        packages: Vec<String>,

        /// Update from Brewfile
        #[arg(long = "from-brewfile")]
        brewfile: Option<PathBuf>,

        /// Show what would be updated without actually updating
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove old versions from mirror
    Prune {
        /// Path to the mirror directory
        path: PathBuf,

        /// Number of versions to keep per package
        #[arg(long, default_value = "1")]
        keep: usize,

        /// Show what would be removed without removing
        #[arg(long)]
        dry_run: bool,
    },

    /// Verify mirror integrity
    Verify {
        /// Path to the mirror directory
        path: PathBuf,

        /// Specific packages to verify (default: all)
        packages: Vec<String>,

        /// Show verbose output
        #[arg(long, short)]
        verbose: bool,
    },
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        MirrorCommand::Create {
            output,
            packages,
            all_installed,
            brewfile,
            casks,
            linux_apps,
            platforms,
            all_platforms,
            no_deps,
            dry_run,
        } => {
            run_create(
                output,
                packages,
                all_installed,
                brewfile,
                casks,
                linux_apps,
                platforms,
                all_platforms,
                no_deps,
                dry_run,
            )
            .await
        }

        MirrorCommand::Serve {
            path,
            port,
            bind,
            log_access,
        } => run_serve(path, port, bind, log_access).await,

        MirrorCommand::Info { path, json } => run_info(path, json).await,

        MirrorCommand::Outdated { path, json } => run_outdated(path, json).await,

        MirrorCommand::Update {
            path,
            packages,
            brewfile,
            dry_run,
        } => run_update(path, packages, brewfile, dry_run).await,

        MirrorCommand::Prune {
            path,
            keep,
            dry_run,
        } => run_prune(path, keep, dry_run).await,

        MirrorCommand::Verify {
            path,
            packages,
            verbose,
        } => run_verify(path, packages, verbose).await,
    }
}

async fn run_create(
    output: PathBuf,
    packages: Vec<String>,
    all_installed: bool,
    brewfile: Option<PathBuf>,
    casks: Vec<String>,
    linux_apps: Vec<String>,
    platforms: Vec<String>,
    all_platforms: bool,
    no_deps: bool,
    dry_run: bool,
) -> Result<()> {
    let paths = Paths::default();
    let db = Database::open(paths.index_db())?;

    // Collect packages
    let mut pkgs = packages;

    if all_installed {
        let installed = stout_state::InstalledPackages::load(&paths)?;
        for name in installed.names() {
            pkgs.push(name.clone());
        }
    }

    if pkgs.is_empty() && casks.is_empty() && linux_apps.is_empty() {
        bail!("No packages specified. Use --all-installed or provide package names.");
    }

    // Determine platforms
    let target_platforms = if all_platforms {
        vec![
            "arm64_sonoma".to_string(),
            "arm64_ventura".to_string(),
            "arm64_monterey".to_string(),
            "x86_64_sonoma".to_string(),
            "x86_64_ventura".to_string(),
            "x86_64_monterey".to_string(),
            "x86_64_linux".to_string(),
            "aarch64_linux".to_string(),
        ]
    } else if platforms.is_empty() {
        vec![detect_platform()]
    } else {
        platforms
    };

    println!(
        "\n{} mirror at {}\n",
        style("Creating").cyan().bold(),
        output.display()
    );

    if dry_run {
        println!("{}", style("DRY RUN - no files will be written").yellow());
    }

    println!("  Formulas: {}", pkgs.len());
    println!("  Casks: {}", casks.len());
    println!("  Linux apps: {}", linux_apps.len());
    println!("  Platforms: {:?}", target_platforms);
    println!("  Include deps: {}", !no_deps);
    println!();

    if dry_run {
        // Show what would be included
        println!("{}", style("Would include:").bold());
        for pkg in &pkgs {
            println!("  - {}", pkg);
        }
        return Ok(());
    }

    let config = MirrorConfig {
        output,
        packages: pkgs,
        casks,
        linux_apps,
        platforms: target_platforms,
        include_deps: !no_deps,
        brewfile,
    };

    let manifest = create_mirror(config, &db).await?;

    println!();
    println!("{}", style("Mirror created successfully!").green().bold());
    println!(
        "  {} formulas",
        manifest.formulas.count
    );
    println!(
        "  {} total size",
        format_size(manifest.total_size, BINARY)
    );

    Ok(())
}

async fn run_serve(path: PathBuf, port: u16, bind: String, log_access: bool) -> Result<()> {
    if !path.exists() {
        bail!("Mirror directory not found: {}", path.display());
    }

    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json at {}", path.display());
    }

    println!(
        "\n{} mirror from {}\n",
        style("Serving").cyan().bold(),
        path.display()
    );

    let config = ServeConfig {
        mirror_path: path,
        port,
        bind,
        log_access,
    };

    serve_mirror(config).await?;

    Ok(())
}

async fn run_info(path: PathBuf, json: bool) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json");
    }

    let manifest = MirrorManifest::load(&manifest_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
    } else {
        println!("\n{} Information\n", style("Mirror").cyan().bold());
        println!("  Version: {}", manifest.version);
        println!("  Created: {}", manifest.created_at);
        println!("  stout version: {}", manifest.stout_version);
        println!();
        println!("  Formulas: {}", manifest.formulas.count);
        println!("  Casks: {}", manifest.casks.count);
        println!("  Linux apps: {}", manifest.linux_apps.count);
        println!();
        println!("  Platforms: {:?}", manifest.platforms);
        println!(
            "  Total size: {}",
            format_size(manifest.total_size, BINARY)
        );
        println!();
    }

    Ok(())
}

async fn run_outdated(path: PathBuf, json: bool) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json");
    }

    let manifest = MirrorManifest::load(&manifest_path)?;
    let paths = Paths::default();
    let db = Database::open(paths.index_db())?;

    println!(
        "\n{} for outdated packages in mirror...\n",
        style("Checking").cyan().bold()
    );

    let mut outdated = Vec::new();

    for (name, info) in &manifest.formulas.packages {
        if let Ok(Some(formula)) = db.get_formula(name) {
            if formula.version != info.version {
                outdated.push((name.clone(), info.version.clone(), formula.version.clone()));
            }
        }
    }

    if json {
        let output: Vec<_> = outdated
            .iter()
            .map(|(name, mirror_ver, latest_ver)| {
                serde_json::json!({
                    "name": name,
                    "mirror_version": mirror_ver,
                    "latest_version": latest_ver
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if outdated.is_empty() {
        println!("{}", style("All packages are up to date!").green());
    } else {
        println!("{} packages outdated:\n", outdated.len());
        for (name, mirror_ver, latest_ver) in &outdated {
            println!(
                "  {} {} -> {}",
                style(name).white().bold(),
                style(mirror_ver).red(),
                style(latest_ver).green()
            );
        }
        println!();
        println!(
            "Run '{}' to update",
            style(format!("stout mirror update {}", path.display())).cyan()
        );
    }

    Ok(())
}

async fn run_update(
    path: PathBuf,
    packages: Vec<String>,
    _brewfile: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json");
    }

    println!(
        "\n{} mirror at {}\n",
        style("Updating").cyan().bold(),
        path.display()
    );

    if dry_run {
        println!("{}", style("DRY RUN - no files will be modified").yellow());
    }

    // TODO: Implement actual update logic
    println!("{}", style("Mirror update not yet implemented").yellow());

    Ok(())
}

async fn run_prune(path: PathBuf, keep: usize, dry_run: bool) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json");
    }

    println!(
        "\n{} mirror at {} (keeping {} versions)\n",
        style("Pruning").cyan().bold(),
        path.display(),
        keep
    );

    if dry_run {
        println!("{}", style("DRY RUN - no files will be removed").yellow());
    }

    // TODO: Implement actual prune logic
    println!("{}", style("Mirror prune not yet implemented").yellow());

    Ok(())
}

async fn run_verify(path: PathBuf, packages: Vec<String>, verbose: bool) -> Result<()> {
    let manifest_path = path.join("manifest.json");
    if !manifest_path.exists() {
        bail!("Invalid mirror: missing manifest.json");
    }

    let manifest = MirrorManifest::load(&manifest_path)?;

    println!(
        "\n{} mirror at {}\n",
        style("Verifying").cyan().bold(),
        path.display()
    );

    let packages_to_check: Vec<&String> = if packages.is_empty() {
        manifest.formulas.packages.keys().collect()
    } else {
        packages.iter().collect()
    };

    let mut errors = 0;
    let mut verified = 0;

    // Check manifest
    print!("  Checking manifest.json... ");
    println!("{}", style("✓").green());
    verified += 1;

    // Check formula bottles
    for name in packages_to_check {
        if let Some(info) = manifest.formulas.packages.get(name) {
            for (platform, bottle) in &info.bottles {
                let bottle_path = path.join(&bottle.path);
                if verbose {
                    print!("  Checking {}/{} bottle... ", name, platform);
                }

                if bottle_path.exists() {
                    // TODO: Verify checksum
                    if verbose {
                        println!("{}", style("✓").green());
                    }
                    verified += 1;
                } else {
                    if verbose {
                        println!("{}", style("✗ missing").red());
                    }
                    errors += 1;
                }
            }
        }
    }

    println!();
    if errors == 0 {
        println!(
            "{} {} files verified",
            style("✓").green(),
            verified
        );
    } else {
        println!(
            "{} {} files verified, {} errors",
            style("!").yellow(),
            verified,
            errors
        );
    }
    println!();

    if errors > 0 {
        bail!("Mirror verification failed with {} errors", errors);
    }

    Ok(())
}
