//! Config command - show brewx configuration

use anyhow::Result;
use brewx_state::{Config, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {}

pub async fn run(_args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    println!("{}", style("BREWX_VERSION").green().bold());
    println!("  {}", env!("CARGO_PKG_VERSION"));

    println!("\n{}", style("ORIGIN").green().bold());
    println!("  {}", "https://github.com/neul-labs/brewx");

    println!("\n{}", style("HOMEBREW_PREFIX").green().bold());
    println!("  {}", paths.prefix.display());

    println!("\n{}", style("HOMEBREW_CELLAR").green().bold());
    println!("  {}", paths.cellar.display());

    println!("\n{}", style("BREWX_DIR").green().bold());
    println!("  {}", paths.brewx_dir.display());

    println!("\n{}", style("BREWX_CACHE").green().bold());
    println!("  {}", paths.brewx_dir.join("downloads").display());

    println!("\n{}", style("INDEX_URL").green().bold());
    println!("  {}", config.index.base_url);

    println!("\n{}", style("CONFIG_FILE").green().bold());
    println!("  {}", paths.config_file().display());

    println!("\n{}", style("INSTALLED_FILE").green().bold());
    println!("  {}", paths.installed_file().display());

    println!("\n{}", style("INDEX_DB").green().bold());
    println!("  {}", paths.index_db().display());

    // System info
    println!("\n{}", style("CPU").green().bold());
    println!("  {}", std::env::consts::ARCH);

    println!("\n{}", style("OS").green().bold());
    println!("  {}", std::env::consts::OS);

    #[cfg(target_os = "macos")]
    {
        println!("\n{}", style("MACOS_VERSION").green().bold());
        if let Ok(output) = std::process::Command::new("sw_vers")
            .args(["-productVersion"])
            .output()
        {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("  {}", version.trim());
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("\n{}", style("LINUX_DISTRO").green().bold());
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("PRETTY_NAME=") {
                    let name = line.trim_start_matches("PRETTY_NAME=").trim_matches('"');
                    println!("  {}", name);
                    break;
                }
            }
        }
    }

    // Rust info
    println!("\n{}", style("RUST_VERSION").green().bold());
    println!("  {}", rustc_version());

    // Security info
    println!("\n{}", style("SECURITY").green().bold());
    let sig_status = if config.security.require_signature {
        style("Signatures required").green()
    } else {
        style("Signatures optional").yellow()
    };
    println!("  {}", sig_status);

    if config.security.allow_unsigned {
        println!("  {} {}", style("⚠").yellow(), style("Unsigned indexes allowed").yellow());
    } else {
        println!("  {} {}", style("✓").green(), "Unsigned indexes blocked");
    }

    let max_age_days = config.security.max_signature_age / (24 * 60 * 60);
    println!("  Max signature age: {} days", max_age_days);

    if !config.security.additional_trusted_keys.is_empty() {
        println!("  {} additional trusted keys configured", config.security.additional_trusted_keys.len());
    }

    Ok(())
}

fn rustc_version() -> String {
    // This is set at compile time
    concat!("rustc ", env!("CARGO_PKG_RUST_VERSION"), " (minimum)").to_string()
}
