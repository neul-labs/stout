//! Prefix command - manage multiple package prefixes
//!
//! Allows creating isolated package installations in different directories,
//! useful for project-specific dependencies or testing.

use anyhow::{bail, Context, Result};
use stout_state::Paths;
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<PrefixCommand>,
}

#[derive(Subcommand)]
pub enum PrefixCommand {
    /// Create a new prefix
    Create {
        /// Path for the new prefix
        path: PathBuf,

        /// Force creation even if directory exists
        #[arg(long, short)]
        force: bool,
    },

    /// List all known prefixes
    List,

    /// Remove a prefix
    Remove {
        /// Path to remove
        path: PathBuf,

        /// Also remove all installed packages
        #[arg(long)]
        packages: bool,

        /// Force removal without confirmation
        #[arg(long, short)]
        force: bool,
    },

    /// Show info about a prefix
    Info {
        /// Path to the prefix (default: current)
        path: Option<PathBuf>,
    },

    /// Set the default prefix
    Default {
        /// Path to set as default
        path: PathBuf,
    },
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        Some(PrefixCommand::Create { path, force }) => run_create(path, force).await,
        Some(PrefixCommand::List) => run_list().await,
        Some(PrefixCommand::Remove { path, packages, force }) => run_remove(path, packages, force).await,
        Some(PrefixCommand::Info { path }) => run_info(path).await,
        Some(PrefixCommand::Default { path }) => run_default(path).await,
        None => run_info(None).await,
    }
}

async fn run_create(path: PathBuf, force: bool) -> Result<()> {
    println!(
        "\n{} prefix at {}...\n",
        style("Creating").cyan().bold(),
        path.display()
    );

    // Expand path
    let path = if path.starts_with("~") {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        home.join(path.strip_prefix("~").unwrap())
    } else {
        path.canonicalize().unwrap_or(path)
    };

    // Check if path exists
    if path.exists() && !force {
        if path.join("Cellar").exists() {
            bail!(
                "Prefix already exists at {}. Use --force to reinitialize.",
                path.display()
            );
        } else if !path.read_dir()?.next().is_none() {
            bail!(
                "Directory not empty: {}. Use --force to create prefix anyway.",
                path.display()
            );
        }
    }

    // Create directory structure
    let cellar = path.join("Cellar");
    let bin = path.join("bin");
    let lib = path.join("lib");
    let include = path.join("include");
    let share = path.join("share");
    let etc = path.join("etc");
    let var = path.join("var");

    std::fs::create_dir_all(&cellar)?;
    std::fs::create_dir_all(&bin)?;
    std::fs::create_dir_all(&lib)?;
    std::fs::create_dir_all(&include)?;
    std::fs::create_dir_all(&share)?;
    std::fs::create_dir_all(&etc)?;
    std::fs::create_dir_all(&var)?;

    // Create a prefix marker file
    let marker_path = path.join(".stout-prefix");
    std::fs::write(
        &marker_path,
        format!(
            "# stout prefix created at {}\n# Use: stout --prefix={} install <pkg>\n",
            chrono_lite::now(),
            path.display()
        ),
    )?;

    // Register prefix
    register_prefix(&path)?;

    println!("{}", style("Prefix created successfully!").green().bold());
    println!();
    println!("{}:", style("Structure").bold());
    println!("  Cellar:  {}", cellar.display());
    println!("  bin:     {}", bin.display());
    println!("  lib:     {}", lib.display());
    println!("  include: {}", include.display());
    println!("  share:   {}", share.display());
    println!();
    println!("{}:", style("Usage").bold());
    println!(
        "  stout --prefix={} install <package>",
        path.display()
    );
    println!(
        "  stout --prefix={} list",
        path.display()
    );
    println!();
    println!("{}:", style("Add to PATH").bold());
    println!("  export PATH=\"{}:$PATH\"", bin.display());

    Ok(())
}

async fn run_list() -> Result<()> {
    let prefixes = list_prefixes()?;
    let default_prefix = default_prefix()?;

    println!("\n{}\n", style("Known Prefixes").cyan().bold());

    if prefixes.is_empty() {
        println!("  {}", style("No custom prefixes registered.").dim());
    } else {
        for prefix in &prefixes {
            let is_default = prefix == &default_prefix;
            let exists = prefix.exists();

            if is_default {
                print!("  {} ", style("*").green().bold());
            } else {
                print!("    ");
            }

            if exists {
                println!("{}", prefix.display());
            } else {
                println!(
                    "{} {}",
                    prefix.display(),
                    style("(not found)").red()
                );
            }
        }
    }

    println!();
    println!("Default prefix: {}", default_prefix.display());
    println!();

    Ok(())
}

async fn run_remove(path: PathBuf, remove_packages: bool, force: bool) -> Result<()> {
    let path = if path.starts_with("~") {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        home.join(path.strip_prefix("~").unwrap())
    } else {
        path.canonicalize().unwrap_or(path)
    };

    if !path.exists() {
        // Just unregister
        unregister_prefix(&path)?;
        println!("Prefix unregistered: {}", path.display());
        return Ok(());
    }

    if !force {
        let cellar = path.join("Cellar");
        if cellar.exists() {
            let pkg_count = std::fs::read_dir(&cellar)?.count();
            if pkg_count > 0 {
                bail!(
                    "Prefix has {} installed packages. Use --force to remove anyway, or --packages to remove packages too.",
                    pkg_count
                );
            }
        }
    }

    if remove_packages {
        println!(
            "\n{} prefix and packages at {}...\n",
            style("Removing").red().bold(),
            path.display()
        );
        std::fs::remove_dir_all(&path)?;
    }

    unregister_prefix(&path)?;

    println!("{}", style("Prefix removed.").green());

    Ok(())
}

async fn run_info(path: Option<PathBuf>) -> Result<()> {
    let path = match path {
        Some(p) => {
            if p.starts_with("~") {
                let home = dirs::home_dir().context("Could not determine home directory")?;
                home.join(p.strip_prefix("~").unwrap())
            } else {
                p.canonicalize().unwrap_or(p)
            }
        }
        None => default_prefix()?,
    };

    println!("\n{} {}\n", style("Prefix:").cyan().bold(), path.display());

    if !path.exists() {
        println!("  Status: {}", style("Not found").red());
        return Ok(());
    }

    let cellar = path.join("Cellar");
    let pkg_count = if cellar.exists() {
        std::fs::read_dir(&cellar)?.count()
    } else {
        0
    };

    println!("  Status: {}", style("Active").green());
    println!("  Installed packages: {}", pkg_count);

    // Calculate disk usage
    let mut total_size: u64 = 0;
    if cellar.exists() {
        for entry in walkdir::WalkDir::new(&cellar)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }
    }

    if total_size > 0 {
        println!(
            "  Disk usage: {}",
            humansize::format_size(total_size, humansize::BINARY)
        );
    }

    println!();
    println!("{}:", style("Directories").bold());
    println!("  Cellar:  {}", cellar.display());
    println!("  bin:     {}", path.join("bin").display());
    println!("  lib:     {}", path.join("lib").display());

    Ok(())
}

async fn run_default(path: PathBuf) -> Result<()> {
    let path = if path.starts_with("~") {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        home.join(path.strip_prefix("~").unwrap())
    } else {
        path.canonicalize().unwrap_or(path)
    };

    if !path.exists() {
        bail!("Prefix does not exist: {}", path.display());
    }

    set_default_prefix(&path)?;

    println!(
        "{} Default prefix set to: {}",
        style("✓").green().bold(),
        path.display()
    );

    Ok(())
}

// Helper functions for prefix management

fn prefixes_file() -> Result<PathBuf> {
    let paths = Paths::default();
    Ok(paths.stout_dir.join("prefixes.txt"))
}

fn register_prefix(path: &std::path::Path) -> Result<()> {
    let file_path = prefixes_file()?;
    let mut prefixes = list_prefixes()?;

    if !prefixes.contains(&path.to_path_buf()) {
        prefixes.push(path.to_path_buf());
    }

    let content: String = prefixes
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(file_path, content)?;
    Ok(())
}

fn unregister_prefix(path: &std::path::Path) -> Result<()> {
    let file_path = prefixes_file()?;
    let prefixes = list_prefixes()?;

    let filtered: Vec<_> = prefixes
        .into_iter()
        .filter(|p| p != path)
        .collect();

    let content: String = filtered
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(file_path, content)?;
    Ok(())
}

fn list_prefixes() -> Result<Vec<PathBuf>> {
    let file_path = prefixes_file()?;

    if !file_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&file_path)?;
    let prefixes: Vec<PathBuf> = content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .collect();

    Ok(prefixes)
}

fn default_prefix() -> Result<PathBuf> {
    // Check for environment variable first
    if let Ok(prefix) = std::env::var("STOUT_PREFIX") {
        return Ok(PathBuf::from(prefix));
    }

    // Then check config file
    let paths = Paths::default();
    let default_file = paths.stout_dir.join("default_prefix");

    if default_file.exists() {
        let content = std::fs::read_to_string(&default_file)?;
        return Ok(PathBuf::from(content.trim()));
    }

    // Fall back to system default
    Ok(paths.prefix.clone())
}

fn set_default_prefix(path: &std::path::Path) -> Result<()> {
    let paths = Paths::default();
    let default_file = paths.stout_dir.join("default_prefix");

    std::fs::write(default_file, path.display().to_string())?;
    Ok(())
}

// Simple timestamp for marker file
mod chrono_lite {
    use std::time::SystemTime;

    pub fn now() -> String {
        let duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();

        // Simple formatting: YYYY-MM-DD HH:MM:SS UTC
        let days_since_epoch = secs / 86400;
        let remaining_secs = secs % 86400;
        let hours = remaining_secs / 3600;
        let minutes = (remaining_secs % 3600) / 60;
        let seconds = remaining_secs % 60;

        // Approximate year calculation (ignoring leap years for simplicity)
        let years = 1970 + (days_since_epoch / 365);

        format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            years,
            1,
            1,
            hours,
            minutes,
            seconds
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chrono_lite_now() {
        let timestamp = chrono_lite::now();
        // Should be in format "YYYY-01-01 HH:MM:SS UTC"
        assert!(timestamp.contains("UTC"));
        assert!(timestamp.len() > 10);
    }

    #[test]
    fn test_expand_tilde_path() {
        let path = PathBuf::from("~/test/path");
        // Just verify it doesn't panic
        if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                let expanded = home.join(path.strip_prefix("~").unwrap());
                assert!(!expanded.starts_with("~"));
            }
        }
    }
}
