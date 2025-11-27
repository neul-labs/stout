//! Home command - open the homepage of a package in the browser

use anyhow::{bail, Context, Result};
use brewx_index::{Database, IndexSync};
use brewx_state::{Config, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Formula or cask to open homepage for
    pub formula: Option<String>,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let config = Config::load(&paths)?;

    // If no formula specified, open brewx/homebrew homepage
    let url = if let Some(ref name) = args.formula {
        let db = Database::open(paths.index_db())
            .context("Failed to open index. Run 'brewx update' first.")?;

        if !db.is_initialized()? {
            bail!("Index not initialized. Run 'brewx update' first.");
        }

        // Try to get formula info
        let sync = IndexSync::with_security_policy(
            Some(&config.index.base_url),
            &paths.brewx_dir,
            config.security.to_security_policy(),
        )?;

        // Try as formula first
        if let Ok(formula) = sync.fetch_formula_cached(name, None).await {
            if let Some(homepage) = formula.homepage {
                homepage
            } else {
                bail!("Formula '{}' has no homepage", name);
            }
        } else if let Ok(cask) = sync.fetch_cask_cached(name, None).await {
            // Try as cask
            if let Some(homepage) = cask.homepage {
                homepage
            } else {
                bail!("Cask '{}' has no homepage", name);
            }
        } else {
            bail!("Formula or cask '{}' not found", name);
        }
    } else {
        // Default to Homebrew homepage
        "https://brew.sh".to_string()
    };

    println!(
        "{} Opening {}",
        style("==>").blue().bold(),
        style(&url).cyan().underlined()
    );

    open_url(&url)?;

    Ok(())
}

/// Open a URL in the default browser
fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .context("Failed to open URL")?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try xdg-open first, then common browsers
        if std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_err()
        {
            // Fallback to common browsers
            for browser in &["firefox", "chromium", "google-chrome", "brave"] {
                if std::process::Command::new(browser)
                    .arg(url)
                    .spawn()
                    .is_ok()
                {
                    return Ok(());
                }
            }
            bail!("Could not find a browser to open URL. Install xdg-utils or a web browser.");
        }
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .context("Failed to open URL")?;
    }

    Ok(())
}
