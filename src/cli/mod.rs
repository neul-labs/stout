//! CLI command definitions and handlers

pub mod completions;
pub mod doctor;
pub mod info;
pub mod install;
pub mod list;
pub mod search;
pub mod uninstall;
pub mod update;
pub mod upgrade;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "brewx",
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
}

#[derive(Subcommand)]
pub enum Command {
    /// Install packages
    Install(install::Args),

    /// Uninstall packages
    Uninstall(uninstall::Args),

    /// Search for packages
    Search(search::Args),

    /// Show package information
    Info(info::Args),

    /// List installed packages
    List(list::Args),

    /// Update the formula index
    Update(update::Args),

    /// Upgrade installed packages
    Upgrade(upgrade::Args),

    /// Check system health
    Doctor(doctor::Args),

    /// Generate shell completions
    Completions(completions::Args),
}
