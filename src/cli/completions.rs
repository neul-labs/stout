//! Shell completions command

use anyhow::Result;
use clap::{Args as ClapArgs, CommandFactory};
use clap_complete::{generate, Shell};
use std::io;

#[derive(ClapArgs)]
pub struct Args {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: Shell,
}

pub async fn run(args: Args) -> Result<()> {
    let mut cmd = super::Cli::command();
    generate(args.shell, &mut cmd, "stout", &mut io::stdout());
    Ok(())
}
