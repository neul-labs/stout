//! Lock command for managing lockfiles

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::path::PathBuf;
use stout_state::{InstalledPackages, LockedPackage, Lockfile, Paths};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<LockCommand>,
}

#[derive(Subcommand)]
pub enum LockCommand {
    /// Generate a lockfile from currently installed packages
    Generate {
        /// Output file path (default: stout.lock)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Install packages from a lockfile
    Install {
        /// Lockfile path (default: stout.lock)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Show lockfile contents
    Show {
        /// Lockfile path (default: stout.lock)
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        Some(LockCommand::Generate { output }) => generate_lockfile(output).await,
        Some(LockCommand::Install { file }) => install_from_lockfile(file).await,
        Some(LockCommand::Show { file }) => show_lockfile(file).await,
        None => show_lockfile(None).await,
    }
}

async fn generate_lockfile(output: Option<PathBuf>) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let mut lockfile = Lockfile::new();

    println!("{}...", style("Generating lockfile").cyan());

    for name in installed.names() {
        let pkg = installed
            .get(name)
            .with_context(|| format!("package '{}' is in installed list but not found", name))?;

        // Create a basic locked package entry
        // In a full implementation, we'd look up bottle/source info
        let locked = LockedPackage {
            version: pkg.version.clone(),
            revision: pkg.revision,
            bottle_sha256: None,
            bottle_url: None,
            source_sha256: None,
            source_url: None,
            built_from_source: false,
            dependencies: vec![],
        };

        lockfile.add_package(name, locked);
    }

    let output_path = output.unwrap_or_else(|| PathBuf::from("stout.lock"));
    lockfile.save(&output_path)?;

    println!(
        "\n{} Generated lockfile with {} packages",
        style("✓").green(),
        lockfile.packages.len()
    );
    println!("  {}: {}", style("Output").dim(), output_path.display());

    Ok(())
}

async fn install_from_lockfile(file: Option<PathBuf>) -> Result<()> {
    let lockfile_path = file.unwrap_or_else(|| PathBuf::from("stout.lock"));

    let lockfile = Lockfile::load(&lockfile_path)
        .context(format!("Failed to load lockfile from {:?}", lockfile_path))?;

    println!(
        "{}...",
        style(format!(
            "Installing from lockfile ({})",
            lockfile_path.display()
        ))
        .cyan()
    );

    if !lockfile.matches_platform() {
        println!(
            "\n{} Lockfile was created for {}, current platform is {}-{}",
            style("warning:").yellow(),
            lockfile.platform,
            std::env::consts::OS,
            std::env::consts::ARCH
        );
    }

    println!("\n{} packages to install:\n", lockfile.packages.len());

    for (name, pkg) in &lockfile.packages {
        let source = if pkg.built_from_source {
            "(source)"
        } else {
            "(bottle)"
        };
        println!(
            "  {} {} {} {}",
            style("•").dim(),
            name,
            style(&pkg.version).dim(),
            style(source).dim()
        );
    }

    println!(
        "\n{}\n",
        style("Run 'stout install <packages>' with the locked versions").dim()
    );

    // In a full implementation, we would actually install the packages
    // with the exact versions from the lockfile

    Ok(())
}

async fn show_lockfile(file: Option<PathBuf>) -> Result<()> {
    let lockfile_path = file.unwrap_or_else(|| PathBuf::from("stout.lock"));

    let lockfile = Lockfile::load(&lockfile_path)
        .context(format!("Failed to load lockfile from {:?}", lockfile_path))?;

    println!("\n{}", style("Lockfile contents").cyan());
    println!("  {}: {}", style("Version").dim(), lockfile.version);
    println!("  {}: {}", style("Platform").dim(), lockfile.platform);
    println!("  {}: {}", style("Created").dim(), lockfile.created_at);
    println!("  {}: {}", style("Packages").dim(), lockfile.packages.len());

    if !lockfile.packages.is_empty() {
        println!("\n{}:\n", style("Packages").cyan());

        for (name, pkg) in &lockfile.packages {
            let source = if pkg.built_from_source {
                "source"
            } else {
                "bottle"
            };
            println!(
                "  {} {} ({}, rev {})",
                style(name).green(),
                style(&pkg.version).dim(),
                source,
                pkg.revision
            );

            if !pkg.dependencies.is_empty() {
                println!(
                    "    {}: {}",
                    style("deps").dim(),
                    pkg.dependencies.join(", ")
                );
            }
        }
    }

    println!();
    Ok(())
}
