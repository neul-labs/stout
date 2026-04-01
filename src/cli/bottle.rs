//! Bottle command - create bottles from installed packages

use anyhow::{bail, Context, Result};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use flate2::read::GzDecoder;
use humansize::{format_size, BINARY};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use stout_install::create_bottle;
use stout_state::{InstalledPackages, Paths};
use tar::Archive;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: BottleCommand,
}

#[derive(Subcommand)]
pub enum BottleCommand {
    /// Create a bottle from an installed package
    Create {
        /// Package name to create bottle from
        package: String,

        /// Output directory (default: current directory)
        #[arg(long, short)]
        output: Option<PathBuf>,

        /// Skip rebuild check
        #[arg(long)]
        no_rebuild: bool,

        /// Force overwrite existing bottle
        #[arg(long, short)]
        force: bool,

        /// Create bottle for all installed versions
        #[arg(long)]
        all_versions: bool,
    },

    /// Show information about an existing bottle
    Info {
        /// Path to the bottle file
        bottle: PathBuf,
    },

    /// Verify a bottle's integrity
    Verify {
        /// Path to the bottle file
        bottle: PathBuf,
    },
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        BottleCommand::Create {
            package,
            output,
            no_rebuild,
            force,
            all_versions,
        } => run_create(package, output, no_rebuild, force, all_versions).await,
        BottleCommand::Info { bottle } => run_info(bottle).await,
        BottleCommand::Verify { bottle } => run_verify(bottle).await,
    }
}

async fn run_create(
    package: String,
    output: Option<PathBuf>,
    _no_rebuild: bool,
    force: bool,
    _all_versions: bool,
) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    // Check if package is installed
    let pkg_info = installed
        .get(&package)
        .ok_or_else(|| anyhow::anyhow!("Package '{}' is not installed", package))?;

    println!(
        "\n{} bottle for {} {}...\n",
        style("Creating").cyan().bold(),
        package,
        pkg_info.version
    );

    // Determine output path
    let output_dir = output.unwrap_or_else(|| PathBuf::from("."));
    let platform = detect_platform();
    let bottle_name = format!(
        "{}-{}.{}.bottle.tar.gz",
        package, pkg_info.version, platform
    );
    let bottle_path = output_dir.join(&bottle_name);

    if bottle_path.exists() && !force {
        bail!(
            "Bottle already exists at {}. Use --force to overwrite.",
            bottle_path.display()
        );
    }

    // Get the installed package path
    let pkg_path = paths.cellar.join(&package).join(&pkg_info.version);
    if !pkg_path.exists() {
        bail!("Package installation not found at {}", pkg_path.display());
    }

    // Create the bottle
    let result = create_bottle(&pkg_path, &bottle_path, &package, &pkg_info.version)
        .context("Failed to create bottle")?;

    println!("{}", style("Bottle created successfully!").green().bold());
    println!("  Path: {}", bottle_path.display());
    println!("  Size: {}", format_size(result.size, BINARY));
    println!("  SHA256: {}", result.sha256);
    println!();

    // Print usage hint
    println!("{}", style("To install this bottle:").dim());
    println!("  stout install --bottle {}", bottle_path.display());

    Ok(())
}

async fn run_info(bottle: PathBuf) -> Result<()> {
    if !bottle.exists() {
        bail!("Bottle not found: {}", bottle.display());
    }

    let file = std::fs::File::open(&bottle)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    println!(
        "\n{} {}\n",
        style("Bottle:").cyan().bold(),
        bottle.display()
    );

    // Get file size
    let metadata = std::fs::metadata(&bottle)?;
    println!("  Size: {}", format_size(metadata.len(), BINARY));

    // List contents
    println!("\n{}:", style("Contents").bold());
    let mut file_count = 0;
    let mut total_size = 0u64;

    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?;
        let size = entry.size();
        total_size += size;
        file_count += 1;

        // Only show first 20 entries
        if file_count <= 20 {
            println!("  {} ({})", path.display(), format_size(size, BINARY));
        }
    }

    if file_count > 20 {
        println!("  ... and {} more files", file_count - 20);
    }

    println!("\n  Total files: {}", file_count);
    println!("  Uncompressed size: {}", format_size(total_size, BINARY));
    println!();

    Ok(())
}

async fn run_verify(bottle: PathBuf) -> Result<()> {
    if !bottle.exists() {
        bail!("Bottle not found: {}", bottle.display());
    }

    println!(
        "\n{} {}...\n",
        style("Verifying").cyan().bold(),
        bottle.display()
    );

    // Calculate SHA256
    let file_bytes = std::fs::read(&bottle)?;
    let mut hasher = Sha256::new();
    hasher.update(&file_bytes);
    let hash = hex::encode(hasher.finalize());

    println!("  SHA256: {}", hash);

    // Verify archive integrity
    print!("  Archive integrity: ");
    let file = std::fs::File::open(&bottle)?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    let mut errors = 0;
    for entry in archive.entries()? {
        if entry.is_err() {
            errors += 1;
        }
    }

    if errors == 0 {
        println!("{}", style("OK").green());
    } else {
        println!("{} ({} errors)", style("FAILED").red(), errors);
    }

    println!();

    if errors > 0 {
        bail!("Bottle verification failed");
    }

    Ok(())
}

fn detect_platform() -> String {
    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };

    if cfg!(target_os = "macos") {
        format!("{}_sonoma", arch)
    } else {
        format!("{}_linux", arch)
    }
}
