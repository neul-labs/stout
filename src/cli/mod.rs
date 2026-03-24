//! CLI command definitions and handlers

pub mod analytics;
pub mod audit;
pub mod autoremove;
pub mod bottle;
pub mod bundle;
pub mod cask;
pub mod cleanup;
pub mod completions;
pub mod config;
pub mod create;
pub mod deps;
pub mod doctor;
pub mod first_run;
pub mod history;
pub mod home;
pub mod import;
pub mod info;
pub mod install;
pub mod link;
pub mod list;
pub mod lock;
pub mod mirror;
pub mod outdated;
pub mod pin;
pub mod prefix;
pub mod reinstall;
pub mod rollback;
pub mod search;
pub mod services;
pub mod snapshot;
pub mod switch;
pub mod sync;
pub mod tap;
pub mod test;
pub mod uninstall;
pub mod unlink;
pub mod unpin;
pub mod update;
pub mod upgrade;
pub mod uses;
pub mod why;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "stout",
    about = "A fast, Rust-based Homebrew-compatible package manager",
    version,
    author
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Use a custom installation prefix
    #[arg(long, global = true, env = "STOUT_PREFIX")]
    pub prefix: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install packages
    Install(install::Args),

    /// Uninstall packages
    Uninstall(uninstall::Args),

    /// Reinstall packages
    Reinstall(reinstall::Args),

    /// Search for packages
    Search(search::Args),

    /// Show package information
    Info(info::Args),

    /// List installed packages
    List(list::Args),

    /// Show outdated packages
    Outdated(outdated::Args),

    /// Update the formula index
    Update(update::Args),

    /// Upgrade installed packages
    Upgrade(upgrade::Args),

    /// Remove unused dependencies
    Autoremove(autoremove::Args),

    /// Remove old downloads and cache files
    Cleanup(cleanup::Args),

    /// Show dependencies of a package
    Deps(deps::Args),

    /// Show packages that depend on a given package
    Uses(uses::Args),

    /// Show why a package is installed (reverse dependency chain)
    Why(why::Args),

    /// Show package version history
    History(history::Args),

    /// Rollback a package to a previous version
    Rollback(rollback::Args),

    /// Switch between installed versions of a package
    Switch(switch::Args),

    /// Pin packages to prevent upgrades
    Pin(pin::Args),

    /// Unpin packages to allow upgrades
    Unpin(unpin::Args),

    /// Link a package (create symlinks)
    Link(link::Args),

    /// Unlink a package (remove symlinks)
    Unlink(unlink::Args),

    /// Open package homepage in browser
    Home(home::Args),

    /// Import existing Homebrew packages into Stout tracking
    Import(import::Args),

    /// Manage taps (custom formula repositories)
    Tap(tap::Args),

    /// Manage lockfiles for reproducible environments
    Lock(lock::Args),

    /// Manage background services
    Services(services::Args),

    /// Check system health
    Doctor(doctor::Args),

    /// Show stout configuration
    Config(config::Args),

    /// Generate shell completions
    Completions(completions::Args),

    /// Manage casks (applications)
    Cask(cask::Args),

    /// Manage Brewfile bundles
    Bundle(bundle::Args),

    /// Manage system snapshots
    Snapshot(snapshot::Args),

    /// Reconcile Stout state with the Homebrew Cellar
    Sync(sync::Args),

    /// Audit packages for known vulnerabilities
    Audit(audit::Args),

    /// Manage offline mirrors
    Mirror(mirror::Args),

    /// Create and manage bottles (binary packages)
    Bottle(bottle::Args),

    /// Create a new formula or cask from a URL
    Create(create::Args),

    /// Test installed formulas
    Test(test::Args),

    /// Manage anonymous usage analytics
    Analytics(analytics::Args),

    /// Manage multiple installation prefixes
    Prefix(prefix::Args),
}
