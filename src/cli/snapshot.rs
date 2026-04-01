//! Snapshot command - Save and restore system state

use anyhow::{bail, Result};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::io::{self, Read, Write};
use stout_bundle::{Snapshot, SnapshotManager};
use stout_cask::InstalledCasks;
use stout_state::{InstalledPackages, Paths};

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: SnapshotCommand,
}

#[derive(Subcommand)]
pub enum SnapshotCommand {
    /// Create a new snapshot
    Create(CreateArgs),

    /// List all snapshots
    List(ListArgs),

    /// Show snapshot details
    Show(ShowArgs),

    /// Restore a snapshot
    Restore(RestoreArgs),

    /// Delete a snapshot
    Delete(DeleteArgs),

    /// Export a snapshot to stdout
    Export(ExportArgs),

    /// Import a snapshot from stdin
    Import(ImportArgs),
}

#[derive(ClapArgs)]
pub struct CreateArgs {
    /// Name for the snapshot
    pub name: String,

    /// Optional description
    #[arg(long, short)]
    pub description: Option<String>,

    /// Overwrite if snapshot exists
    #[arg(long, short)]
    pub force: bool,
}

#[derive(ClapArgs)]
pub struct ListArgs {
    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct ShowArgs {
    /// Snapshot name
    pub name: String,

    /// Show JSON output
    #[arg(long)]
    pub json: bool,
}

#[derive(ClapArgs)]
pub struct RestoreArgs {
    /// Snapshot name
    pub name: String,

    /// Don't actually restore, just show what would be done
    #[arg(long)]
    pub dry_run: bool,

    /// Force install even if already installed
    #[arg(long, short)]
    pub force: bool,
}

#[derive(ClapArgs)]
pub struct DeleteArgs {
    /// Snapshot name
    pub name: String,

    /// Don't ask for confirmation
    #[arg(long, short)]
    pub force: bool,
}

#[derive(ClapArgs)]
pub struct ExportArgs {
    /// Snapshot name
    pub name: String,
}

#[derive(ClapArgs)]
pub struct ImportArgs {
    /// Optional name override (default: use name from snapshot)
    #[arg(long)]
    pub name: Option<String>,

    /// Overwrite if exists
    #[arg(long, short)]
    pub force: bool,
}

pub async fn run(args: Args) -> Result<()> {
    match args.command {
        SnapshotCommand::Create(args) => run_create(args).await,
        SnapshotCommand::List(args) => run_list(args).await,
        SnapshotCommand::Show(args) => run_show(args).await,
        SnapshotCommand::Restore(args) => run_restore(args).await,
        SnapshotCommand::Delete(args) => run_delete(args).await,
        SnapshotCommand::Export(args) => run_export(args).await,
        SnapshotCommand::Import(args) => run_import(args).await,
    }
}

async fn run_create(args: CreateArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    // Check if exists
    if manager.exists(&args.name) && !args.force {
        bail!(
            "Snapshot '{}' already exists. Use --force to overwrite.",
            args.name
        );
    }

    // Create snapshot
    let mut snapshot = Snapshot::new(&args.name, args.description.as_deref());

    // Add installed formulas
    let installed = InstalledPackages::load(&paths)?;
    for (name, pkg) in installed.iter() {
        snapshot.add_formula(name, &pkg.version, pkg.revision, pkg.requested);
    }

    // Add installed casks
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;
    for (token, cask) in installed_casks.iter() {
        snapshot.add_cask(token, &cask.version);
    }

    // Add pinned packages
    for (name, pkg) in installed.iter() {
        if pkg.pinned {
            snapshot.pinned.push(name.clone());
        }
    }

    // Save snapshot
    let path = manager.save(&snapshot)?;

    println!(
        "{} Created snapshot '{}' with {} formulas and {} casks",
        style("✓").green(),
        args.name,
        snapshot.formula_count(),
        snapshot.cask_count()
    );
    println!("  Saved to: {}", path.display());

    Ok(())
}

async fn run_list(args: ListArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    let snapshots = manager.list()?;

    if snapshots.is_empty() {
        println!("No snapshots found.");
        println!(
            "{}",
            style("Use 'stout snapshot create <name>' to create one.").dim()
        );
        return Ok(());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&snapshots)?);
        return Ok(());
    }

    println!("{} snapshot(s):\n", snapshots.len());

    for snap in snapshots {
        print!("{} ", style(&snap.name).green().bold());
        print!(
            "({} formulas, {} casks)",
            snap.formula_count, snap.cask_count
        );
        println!(" {}", style(&snap.created_at).dim());

        if let Some(desc) = &snap.description {
            println!("  {}", style(desc).dim());
        }
    }

    Ok(())
}

async fn run_show(args: ShowArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    let snapshot = manager.load(&args.name)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
        return Ok(());
    }

    println!("{}: {}", style("Name").bold(), snapshot.name);
    println!("{}: {}", style("Created").bold(), snapshot.created_at);
    println!(
        "{}: {}",
        style("stout version").bold(),
        snapshot.stout_version
    );

    if let Some(desc) = &snapshot.description {
        println!("{}: {}", style("Description").bold(), desc);
    }

    println!();

    // Formulas
    if !snapshot.formulas.is_empty() {
        println!(
            "{} ({}):",
            style("Formulas").bold(),
            snapshot.formulas.len()
        );
        for f in &snapshot.formulas {
            let marker = if f.requested { "●" } else { "○" };
            println!(
                "  {} {} {}",
                style(marker).dim(),
                f.name,
                style(&f.version).dim()
            );
        }
        println!();
    }

    // Casks
    if !snapshot.casks.is_empty() {
        println!("{} ({}):", style("Casks").bold(), snapshot.casks.len());
        for c in &snapshot.casks {
            println!(
                "  {} {} {}",
                style("●").dim(),
                c.token,
                style(&c.version).dim()
            );
        }
        println!();
    }

    // Pinned
    if !snapshot.pinned.is_empty() {
        println!("{} ({}):", style("Pinned").bold(), snapshot.pinned.len());
        for name in &snapshot.pinned {
            println!("  {} {}", style("📌").dim(), name);
        }
    }

    Ok(())
}

async fn run_restore(args: RestoreArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    let snapshot = manager.load(&args.name)?;

    println!("{} '{}'...", style("Restoring snapshot").cyan(), args.name);

    let installed = InstalledPackages::load(&paths)?;
    let state_path = paths.stout_dir.join("casks.json");
    let installed_casks = InstalledCasks::load(&state_path)?;

    // Find missing formulas
    let mut missing_formulas = Vec::new();
    for f in &snapshot.formulas {
        if !installed.is_installed(&f.name) {
            missing_formulas.push(&f.name);
        }
    }

    // Find missing casks
    let mut missing_casks = Vec::new();
    for c in &snapshot.casks {
        if !installed_casks.is_installed(&c.token) {
            missing_casks.push(&c.token);
        }
    }

    if missing_formulas.is_empty() && missing_casks.is_empty() {
        println!(
            "{} All packages in snapshot are already installed.",
            style("✓").green()
        );
        return Ok(());
    }

    // Show what would be installed
    if !missing_formulas.is_empty() {
        println!("\n{}:", style("Formulas to install").bold());
        for name in &missing_formulas {
            println!("  {} {}", style("+").green(), name);
        }
    }

    if !missing_casks.is_empty() {
        println!("\n{}:", style("Casks to install").bold());
        for token in &missing_casks {
            println!("  {} {}", style("+").green(), token);
        }
    }

    if args.dry_run {
        println!("\n{}", style("Dry run - no changes made.").yellow());
        return Ok(());
    }

    // Actually restore
    println!(
        "\n{}",
        style("To restore, run the install commands above.").dim()
    );
    println!(
        "{}",
        style("Full restore automation coming in a future update.").dim()
    );

    Ok(())
}

async fn run_delete(args: DeleteArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    if !manager.exists(&args.name) {
        bail!("Snapshot '{}' not found.", args.name);
    }

    if !args.force {
        print!("Delete snapshot '{}'? [y/N] ", args.name);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    manager.delete(&args.name)?;
    println!("{} Deleted snapshot '{}'", style("✓").green(), args.name);

    Ok(())
}

async fn run_export(args: ExportArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    let json = manager.export(&args.name)?;
    println!("{}", json);

    Ok(())
}

async fn run_import(args: ImportArgs) -> Result<()> {
    let paths = Paths::default();
    let manager = SnapshotManager::new(&paths.stout_dir);

    // Read from stdin
    let mut json = String::new();
    io::stdin().read_to_string(&mut json)?;

    // Parse to validate
    let mut snapshot: Snapshot = serde_json::from_str(&json)?;

    // Override name if provided
    if let Some(name) = args.name {
        snapshot.name = name;
    }

    // Check if exists
    if manager.exists(&snapshot.name) && !args.force {
        bail!(
            "Snapshot '{}' already exists. Use --force to overwrite.",
            snapshot.name
        );
    }

    manager.save(&snapshot)?;
    println!(
        "{} Imported snapshot '{}'",
        style("✓").green(),
        snapshot.name
    );

    Ok(())
}
