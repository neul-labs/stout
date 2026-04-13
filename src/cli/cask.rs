//! Cask command - deprecated, delegates to top-level commands

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};
use console::style;

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
    eprintln!(
        "\n{} 'stout cask' is deprecated. Use top-level commands instead:",
        style("Warning:").yellow().bold()
    );

    match args.command {
        CaskCommand::Install(a) => {
            eprintln!("  stout install --cask {}", a.casks.join(" "));
            let install_args = crate::cli::install::Args {
                formulas: a.casks,
                ignore_dependencies: false,
                dry_run: a.dry_run,
                build_from_source: false,
                head: false,
                keep_bottles: false,
                jobs: None,
                cc: None,
                cxx: None,
                force: a.force,
                cask: true,
                formula: false,
                no_verify: a.no_verify,
                appdir: a.appdir,
            };
            crate::cli::install::run(install_args).await
        }
        CaskCommand::Uninstall(a) => {
            eprintln!("  stout uninstall --cask {}", a.casks.join(" "));
            let uninstall_args = crate::cli::uninstall::Args {
                formulas: a.casks,
                force: a.force,
                dry_run: false,
                cask: true,
                formula: false,
                zap: a.zap,
            };
            crate::cli::uninstall::run(uninstall_args).await
        }
        CaskCommand::Info(a) => {
            eprintln!("  stout info --cask {}", a.cask);
            let info_args = crate::cli::info::Args {
                name: a.cask,
                cask: true,
                formula: false,
            };
            crate::cli::info::run(info_args).await
        }
        CaskCommand::Search(a) => {
            eprintln!("  stout search --cask {}", a.query);
            let search_args = crate::cli::search::Args {
                query: a.query,
                limit: 20,
                formula: false,
                cask: true,
            };
            crate::cli::search::run(search_args).await
        }
        CaskCommand::List(a) => {
            eprintln!("  stout list --cask");
            let list_args = crate::cli::list::Args {
                versions: a.versions,
                paths: false,
                source: None,
                requested: false,
                deps: false,
                pinned: false,
                cask: true,
                formula: false,
                json: a.json,
            };
            crate::cli::list::run(list_args).await
        }
        CaskCommand::Outdated(a) => {
            eprintln!("  stout outdated --cask");
            let outdated_args = crate::cli::outdated::Args {
                formulas: Vec::new(),
                verbose: false,
                json: a.json,
                formula: false,
                cask: true,
                greedy: false,
                fetch_head: false,
            };
            crate::cli::outdated::run(outdated_args).await
        }
        CaskCommand::Upgrade(a) => {
            eprintln!("  stout upgrade --cask {}", a.casks.join(" "));
            let upgrade_args = crate::cli::upgrade::Args {
                formulas: a.casks,
                dry_run: a.dry_run,
                fetch_head: false,
                cask: true,
                formula: false,
            };
            crate::cli::upgrade::run(upgrade_args).await
        }
    }
}
