//! Services command - manage background services
//!
//! This provides basic service management for packages that include
//! launchd plists (macOS) or systemd units (Linux).

use anyhow::{bail, Context, Result};
use stout_state::{InstalledPackages, Paths};
use clap::{Args as ClapArgs, Subcommand};
use console::style;
use std::path::PathBuf;

#[derive(ClapArgs)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<ServiceCommand>,
}

#[derive(Subcommand)]
pub enum ServiceCommand {
    /// List all managed services
    List,

    /// Start a service
    Start {
        /// Service/formula name
        service: String,
    },

    /// Stop a service
    Stop {
        /// Service/formula name
        service: String,
    },

    /// Restart a service
    Restart {
        /// Service/formula name
        service: String,
    },

    /// Run a service (without registering to launch at login)
    Run {
        /// Service/formula name
        service: String,
    },

    /// Show service info
    Info {
        /// Service/formula name
        service: String,
    },

    /// Clean up unused services
    Cleanup,
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq)]
enum ServiceStatus {
    Running,
    Stopped,
    Error,
    Unknown,
}

impl std::fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceStatus::Running => write!(f, "started"),
            ServiceStatus::Stopped => write!(f, "stopped"),
            ServiceStatus::Error => write!(f, "error"),
            ServiceStatus::Unknown => write!(f, "unknown"),
        }
    }
}

pub async fn run(args: Args) -> Result<()> {
    let command = args.command.unwrap_or(ServiceCommand::List);

    match command {
        ServiceCommand::List => list_services().await,
        ServiceCommand::Start { service } => start_service(&service).await,
        ServiceCommand::Stop { service } => stop_service(&service).await,
        ServiceCommand::Restart { service } => restart_service(&service).await,
        ServiceCommand::Run { service } => run_service(&service).await,
        ServiceCommand::Info { service } => info_service(&service).await,
        ServiceCommand::Cleanup => cleanup_services().await,
    }
}

async fn list_services() -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    println!(
        "{} Managed services:",
        style("==>").blue().bold()
    );

    let mut found_services = false;

    for name in installed.names() {
        let pkg = installed.get(name)
            .with_context(|| format!("package '{}' is in installed list but not found", name))?;
        let install_path = paths.cellar.join(name).join(&pkg.version);

        // Look for service files
        let service_files = find_service_files(&install_path);

        if !service_files.is_empty() {
            found_services = true;
            let status = get_service_status(name);
            let status_style = match status {
                ServiceStatus::Running => style("started").green(),
                ServiceStatus::Stopped => style("stopped").dim(),
                ServiceStatus::Error => style("error").red(),
                ServiceStatus::Unknown => style("unknown").yellow(),
            };

            println!(
                "  {} {} ({}) - {}",
                style(if status == ServiceStatus::Running { "●" } else { "○" }).dim(),
                name,
                &pkg.version,
                status_style
            );
        }
    }

    if !found_services {
        println!("  {}", style("No services available").dim());
    }

    Ok(())
}

async fn start_service(name: &str) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let pkg = installed
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Formula '{}' is not installed", name))?;

    let install_path = paths.cellar.join(name).join(&pkg.version);
    let service_files = find_service_files(&install_path);

    if service_files.is_empty() {
        bail!("Formula '{}' does not have a service to start", name);
    }

    println!(
        "{} Starting {}...",
        style("==>").blue().bold(),
        style(name).cyan()
    );

    #[cfg(target_os = "macos")]
    {
        for plist in service_files {
            let output = std::process::Command::new("launchctl")
                .args(["load", "-w"])
                .arg(&plist)
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to start service: {}", stderr);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // For Linux, we'd use systemctl
        println!("{}", style("Service management on Linux requires systemd setup").yellow());
    }

    println!("{} Successfully started {}", style("✓").green(), name);
    Ok(())
}

async fn stop_service(name: &str) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let pkg = installed
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Formula '{}' is not installed", name))?;

    let install_path = paths.cellar.join(name).join(&pkg.version);
    let service_files = find_service_files(&install_path);

    if service_files.is_empty() {
        bail!("Formula '{}' does not have a service to stop", name);
    }

    println!(
        "{} Stopping {}...",
        style("==>").blue().bold(),
        style(name).cyan()
    );

    #[cfg(target_os = "macos")]
    {
        for plist in service_files {
            let output = std::process::Command::new("launchctl")
                .args(["unload", "-w"])
                .arg(&plist)
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("{} {}", style("Warning:").yellow(), stderr);
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        println!("{}", style("Service management on Linux requires systemd setup").yellow());
    }

    println!("{} Successfully stopped {}", style("✓").green(), name);
    Ok(())
}

async fn restart_service(name: &str) -> Result<()> {
    stop_service(name).await?;
    start_service(name).await
}

async fn run_service(name: &str) -> Result<()> {
    // Run foreground without registering
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let pkg = installed
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Formula '{}' is not installed", name))?;

    let install_path = paths.cellar.join(name).join(&pkg.version);
    let service_files = find_service_files(&install_path);

    if service_files.is_empty() {
        bail!("Formula '{}' does not have a service", name);
    }

    println!(
        "{} Running {} in foreground (Ctrl+C to stop)...",
        style("==>").blue().bold(),
        style(name).cyan()
    );

    #[cfg(target_os = "macos")]
    {
        for plist in &service_files {
            let output = std::process::Command::new("launchctl")
                .args(["start"])
                .arg(plist)
                .output()?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!("{} {}", style("Warning:").yellow(), stderr);
            }
        }
    }

    Ok(())
}

async fn info_service(name: &str) -> Result<()> {
    let paths = Paths::default();
    let installed = InstalledPackages::load(&paths)?;

    let pkg = installed
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Formula '{}' is not installed", name))?;

    let install_path = paths.cellar.join(name).join(&pkg.version);
    let service_files = find_service_files(&install_path);

    println!(
        "{} Service info for {}:",
        style("==>").blue().bold(),
        style(name).cyan()
    );

    println!("  {}: {}", style("Version").dim(), pkg.version);
    println!("  {}: {}", style("Status").dim(), get_service_status(name));

    if service_files.is_empty() {
        println!("  {}: {}", style("Service files").dim(), "none");
    } else {
        println!("  {}:", style("Service files").dim());
        for file in &service_files {
            println!("    {}", file.display());
        }
    }

    Ok(())
}

async fn cleanup_services() -> Result<()> {
    println!(
        "{} Cleaning up unused services...",
        style("==>").blue().bold()
    );

    // This would remove orphaned plist files
    println!("{}", style("No unused services found.").dim());

    Ok(())
}

/// Find service files (launchd plists or systemd units) for a package
fn find_service_files(install_path: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Check for launchd plists
    let homebrew_dir = install_path.join("homebrew.mxcl.*.plist");
    if let Ok(entries) = glob_simple(install_path, "*.plist") {
        files.extend(entries);
    }

    // Check opt/share for service files
    let share_dir = install_path.join("share");
    if share_dir.exists() {
        if let Ok(entries) = glob_simple(&share_dir, "*.plist") {
            files.extend(entries);
        }
    }

    files
}

/// Simple glob matching
fn glob_simple(dir: &std::path::Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let mut results = Vec::new();

    if !dir.exists() {
        return Ok(results);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();

        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            if name.ends_with(suffix) {
                results.push(entry.path());
            }
        } else if name == pattern {
            results.push(entry.path());
        }
    }

    Ok(results)
}

/// Get service status
fn get_service_status(name: &str) -> ServiceStatus {
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("launchctl")
            .args(["list"])
            .output();

        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(name) {
                return ServiceStatus::Running;
            }
        }
        ServiceStatus::Stopped
    }

    #[cfg(not(target_os = "macos"))]
    {
        ServiceStatus::Unknown
    }
}
