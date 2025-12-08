//! Tap command for managing custom formula repositories

use anyhow::{bail, Context, Result};
use stout_state::{Config, Paths, Tap, TapManager};
use clap::{Args as ClapArgs, Subcommand};
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<TapCommand>,
}

#[derive(Subcommand)]
pub enum TapCommand {
    /// Add a new tap
    Add {
        /// Tap name (e.g., user/repo or full URL)
        name: String,

        /// Custom URL for the tap's index
        #[arg(long)]
        url: Option<String>,
    },

    /// Remove a tap
    Remove {
        /// Tap name to remove
        name: String,
    },

    /// List all taps
    List,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let mut tap_manager = TapManager::load(&paths)?;

    match args.command {
        Some(TapCommand::Add { name, url }) => {
            add_tap(&mut tap_manager, &paths, &name, url.as_deref()).await
        }
        Some(TapCommand::Remove { name }) => {
            remove_tap(&mut tap_manager, &paths, &name)
        }
        Some(TapCommand::List) | None => {
            list_taps(&tap_manager)
        }
    }
}

async fn add_tap(
    manager: &mut TapManager,
    paths: &Paths,
    name: &str,
    custom_url: Option<&str>,
) -> Result<()> {
    // Parse tap name
    let (tap_name, url) = if let Some(url) = custom_url {
        (name.to_string(), url.to_string())
    } else if name.contains('/') && !name.contains("://") {
        // Format: user/repo -> GitHub URL
        let url = format!(
            "https://raw.githubusercontent.com/{}/main",
            name.replace("homebrew-", "")
        );
        (name.to_string(), url)
    } else if name.starts_with("http://") || name.starts_with("https://") {
        // Full URL provided
        let tap_name = name
            .trim_end_matches('/')
            .rsplit('/')
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("/");
        (tap_name, name.to_string())
    } else {
        bail!("Invalid tap format. Use 'user/repo' or provide a URL with --url");
    };

    // Check if already added
    if manager.get(&tap_name).is_some() {
        println!(
            "{} Tap '{}' is already added",
            style("•").dim(),
            tap_name
        );
        return Ok(());
    }

    println!("{}...", style(format!("Adding tap {}", tap_name)).cyan());

    // Verify the tap exists by fetching its manifest
    let client = reqwest::Client::new();
    let manifest_url = format!("{}/manifest.json", url.trim_end_matches('/'));

    let response = client.get(&manifest_url).send().await;
    match response {
        Ok(resp) if resp.status().is_success() => {
            // Tap exists, add it
            let tap = Tap {
                name: tap_name.clone(),
                url: url.clone(),
                pinned: false,
            };

            manager.add(tap);
            manager.save(paths)?;

            println!(
                "\n{} Added tap '{}'",
                style("✓").green(),
                tap_name
            );
            println!("  {}: {}", style("URL").dim(), url);
            println!(
                "\n{}",
                style("Run 'stout update' to sync the new tap").dim()
            );
        }
        Ok(resp) => {
            bail!(
                "Tap '{}' not found at {} (HTTP {})",
                tap_name,
                manifest_url,
                resp.status()
            );
        }
        Err(e) => {
            bail!("Failed to verify tap '{}': {}", tap_name, e);
        }
    }

    Ok(())
}

fn remove_tap(manager: &mut TapManager, paths: &Paths, name: &str) -> Result<()> {
    if manager.get(name).is_none() {
        bail!("Tap '{}' is not installed", name);
    }

    // Don't allow removing the core tap
    if name == "homebrew/core" || name == "neul-labs/stout-index" {
        bail!("Cannot remove the core tap");
    }

    manager.remove(name);
    manager.save(paths)?;

    println!("{} Removed tap '{}'", style("✓").green(), name);

    Ok(())
}

fn list_taps(manager: &TapManager) -> Result<()> {
    let taps = manager.list();

    if taps.is_empty() {
        println!("\n{}", style("No taps configured.").dim());
        println!("{}", style("Run 'stout tap add <name>' to add a tap").dim());
        return Ok(());
    }

    println!("\n{}:\n", style("Installed taps").cyan());

    for tap in taps {
        let pinned = if tap.pinned {
            format!(" {}", style("(pinned)").yellow())
        } else {
            String::new()
        };

        println!("  {} {}{}", style("•").green(), tap.name, pinned);
        println!("    {}", style(&tap.url).dim());
    }

    println!();
    Ok(())
}
