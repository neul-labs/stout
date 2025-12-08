//! stout - A fast, Rust-based Homebrew-compatible package manager

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
                .add_directive("stout=info".parse().unwrap()),
        )
        .without_time()
        .init();

    let cli = Cli::parse();

    // Set STOUT_PREFIX environment variable if custom prefix is specified
    if let Some(ref prefix) = cli.prefix {
        std::env::set_var("STOUT_PREFIX", prefix);
    }

    match cli.command {
        Command::Install(args) => cli::install::run(args).await,
        Command::Uninstall(args) => cli::uninstall::run(args).await,
        Command::Reinstall(args) => cli::reinstall::run(args).await,
        Command::Search(args) => cli::search::run(args).await,
        Command::Info(args) => cli::info::run(args).await,
        Command::List(args) => cli::list::run(args).await,
        Command::Outdated(args) => cli::outdated::run(args).await,
        Command::Update(args) => cli::update::run(args).await,
        Command::Upgrade(args) => cli::upgrade::run(args).await,
        Command::Autoremove(args) => cli::autoremove::run(args).await,
        Command::Cleanup(args) => cli::cleanup::run(args).await,
        Command::Deps(args) => cli::deps::run(args).await,
        Command::Uses(args) => cli::uses::run(args).await,
        Command::Why(args) => cli::why::run(args).await,
        Command::History(args) => cli::history::run(args).await,
        Command::Rollback(args) => cli::rollback::run(args).await,
        Command::Switch(args) => cli::switch::run(args).await,
        Command::Pin(args) => cli::pin::run(args).await,
        Command::Unpin(args) => cli::unpin::run(args).await,
        Command::Link(args) => cli::link::run(args).await,
        Command::Unlink(args) => cli::unlink::run(args).await,
        Command::Home(args) => cli::home::run(args).await,
        Command::Tap(args) => cli::tap::run(args).await,
        Command::Lock(args) => cli::lock::run(args).await,
        Command::Services(args) => cli::services::run(args).await,
        Command::Doctor(args) => cli::doctor::run(args).await,
        Command::Config(args) => cli::config::run(args).await,
        Command::Completions(args) => cli::completions::run(args).await,
        Command::Cask(args) => cli::cask::run(args).await,
        Command::Bundle(args) => cli::bundle::run(args).await,
        Command::Snapshot(args) => cli::snapshot::run(args).await,
        Command::Audit(args) => cli::audit::run(args).await,
        Command::Mirror(args) => cli::mirror::run(args).await,
        Command::Bottle(args) => cli::bottle::run(args).await,
        Command::Create(args) => cli::create::run(args).await,
        Command::Test(args) => cli::test::run(args).await,
        Command::Analytics(args) => cli::analytics::run(args).await,
        Command::Prefix(args) => cli::prefix::run(args).await,
    }
}
