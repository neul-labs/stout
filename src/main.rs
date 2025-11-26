//! brewx - A fast, Rust-based Homebrew-compatible package manager

mod cli;
mod output;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("brewx=info".parse().unwrap()),
        )
        .without_time()
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Install(args) => cli::install::run(args).await,
        Command::Uninstall(args) => cli::uninstall::run(args).await,
        Command::Search(args) => cli::search::run(args).await,
        Command::Info(args) => cli::info::run(args).await,
        Command::List(args) => cli::list::run(args).await,
        Command::Update(args) => cli::update::run(args).await,
        Command::Upgrade(args) => cli::upgrade::run(args).await,
        Command::Doctor(args) => cli::doctor::run(args).await,
        Command::Completions(args) => cli::completions::run(args).await,
    }
}
