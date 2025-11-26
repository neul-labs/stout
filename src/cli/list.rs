//! List command

use anyhow::Result;
use brewx_state::{InstalledPackages, Paths};
use clap::Args as ClapArgs;
use console::style;

#[derive(ClapArgs)]
pub struct Args {
    /// Show versions only
    #[arg(short, long)]
    pub versions: bool,

    /// Show full paths
    #[arg(short, long)]
    pub paths: bool,
}

pub async fn run(args: Args) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    if installed.count() == 0 {
        println!("\n{}", style("No packages installed.").dim());
        return Ok(());
    }

    println!(
        "\n{} ({}):\n",
        style("Installed packages").cyan(),
        installed.count()
    );

    // Collect and sort
    let mut packages: Vec<_> = installed.packages.iter().collect();
    packages.sort_by(|a, b| a.0.cmp(b.0));

    // Find max lengths for alignment
    let max_name = packages.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let max_ver = packages
        .iter()
        .map(|(_, p)| p.version.len())
        .max()
        .unwrap_or(0);

    for (name, pkg) in packages {
        if args.paths {
            let path = paths.package_path(name, &pkg.version);
            println!(
                "  {:<width$}  {}",
                style(name).green(),
                path.display(),
                width = max_name
            );
        } else {
            let req_marker = if pkg.requested { "" } else { " (dep)" };
            println!(
                "  {:<name_w$}  {:<ver_w$}  {}{}",
                style(name).green(),
                style(&pkg.version).dim(),
                style(&pkg.installed_at).dim(),
                style(req_marker).dim(),
                name_w = max_name,
                ver_w = max_ver,
            );
        }
    }

    println!();
    Ok(())
}
