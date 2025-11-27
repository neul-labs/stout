//! Build script for brewx - generates man pages from CLI definition

use clap::{CommandFactory, Parser, Subcommand};
use clap_mangen::Man;
use std::env;
use std::fs;
use std::path::PathBuf;

// Minimal CLI definition for man page generation
// This mirrors src/cli/mod.rs but without the runtime dependencies

#[derive(Parser)]
#[command(
    name = "brewx",
    about = "A fast, Rust-based Homebrew-compatible package manager",
    long_about = "brewx is a drop-in replacement for the Homebrew CLI that's 10-100x faster \
                  for common operations. It uses a pre-computed SQLite index with FTS5 full-text \
                  search, fetches only what it needs, and downloads bottles in parallel.",
    version,
    author = "Neul Labs"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Use a custom installation prefix
    #[arg(long, global = true, env = "BREWX_PREFIX")]
    prefix: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Install packages from bottles or source
    #[command(long_about = "Install one or more packages. By default, brewx installs pre-built \
                            bottles (binary packages) for fast installation. Use --build-from-source \
                            to compile from source instead.")]
    Install {
        /// Package names to install
        packages: Vec<String>,
        /// Build from source instead of using bottles
        #[arg(short = 's', long)]
        build_from_source: bool,
        /// Number of parallel build jobs
        #[arg(short, long)]
        jobs: Option<usize>,
        /// Force reinstall even if already installed
        #[arg(short, long)]
        force: bool,
    },

    /// Uninstall packages
    #[command(long_about = "Remove one or more installed packages. Use --force to remove even \
                            if other packages depend on them.")]
    Uninstall {
        /// Package names to uninstall
        packages: Vec<String>,
        /// Force removal even if dependencies exist
        #[arg(short, long)]
        force: bool,
    },

    /// Reinstall packages
    Reinstall {
        /// Package names to reinstall
        packages: Vec<String>,
    },

    /// Search for packages by name or description
    #[command(long_about = "Search for packages in the formula index. Supports full-text search \
                            across package names and descriptions. Use --desc to search only \
                            descriptions.")]
    Search {
        /// Search query
        query: String,
        /// Search in descriptions only
        #[arg(short, long)]
        desc: bool,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show detailed package information
    #[command(long_about = "Display detailed information about a package including version, \
                            dependencies, installation status, and available bottles.")]
    Info {
        /// Package name
        package: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// List installed packages
    #[command(long_about = "List all installed packages. Use --versions to show version numbers, \
                            --pinned to show only pinned packages.")]
    List {
        /// Show version numbers
        #[arg(long)]
        versions: bool,
        /// Show only pinned packages
        #[arg(long)]
        pinned: bool,
    },

    /// Show outdated packages with available updates
    Outdated {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Update the formula index
    #[command(long_about = "Download the latest formula index from the remote repository. \
                            This updates the local database used for searching and package info.")]
    Update {
        /// Force update even if recently updated
        #[arg(short, long)]
        force: bool,
    },

    /// Upgrade installed packages to latest versions
    #[command(long_about = "Upgrade one or more packages to their latest versions. If no packages \
                            are specified, upgrades all outdated packages.")]
    Upgrade {
        /// Package names to upgrade (all if empty)
        packages: Vec<String>,
        /// Dry run - show what would be upgraded
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Remove unused dependencies (orphans)
    #[command(long_about = "Remove packages that were installed as dependencies but are no longer \
                            needed by any installed package.")]
    Autoremove {
        /// Dry run - show what would be removed
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Remove old downloads and cache files
    #[command(long_about = "Clean up disk space by removing old downloads, outdated formula cache, \
                            and old versions of installed packages.")]
    Cleanup {
        /// Remove all cached files
        #[arg(short, long)]
        all: bool,
        /// Dry run - show what would be removed
        #[arg(short = 'n', long)]
        dry_run: bool,
        /// Number of days to keep cached files
        #[arg(long, default_value = "30")]
        days: u32,
    },

    /// Show dependencies of a package
    #[command(long_about = "Display the dependency tree for a package. Shows both runtime and \
                            build dependencies.")]
    Deps {
        /// Package name
        package: String,
        /// Show tree format
        #[arg(long)]
        tree: bool,
        /// Include build dependencies
        #[arg(long)]
        include_build: bool,
    },

    /// Show reverse dependencies (packages depending on this one)
    Uses {
        /// Package name
        package: String,
        /// Show only installed packages
        #[arg(long)]
        installed: bool,
    },

    /// Show why a package is installed
    Why {
        /// Package name
        package: String,
    },

    /// Show package version history
    History {
        /// Package name (optional, shows all if omitted)
        package: Option<String>,
    },

    /// Rollback a package to a previous version
    Rollback {
        /// Package name
        package: String,
    },

    /// Switch between installed versions of a package
    Switch {
        /// Package name
        package: String,
        /// Version to switch to
        version: String,
    },

    /// Pin packages to prevent automatic upgrades
    Pin {
        /// Package names to pin
        packages: Vec<String>,
    },

    /// Unpin packages to allow upgrades
    Unpin {
        /// Package names to unpin
        packages: Vec<String>,
    },

    /// Link a package (create symlinks in prefix)
    Link {
        /// Package name
        package: String,
        /// Overwrite existing files
        #[arg(long)]
        overwrite: bool,
    },

    /// Unlink a package (remove symlinks, keep installation)
    Unlink {
        /// Package name
        package: String,
    },

    /// Open package homepage in default browser
    Home {
        /// Package name
        package: String,
    },

    /// Manage taps (custom formula repositories)
    #[command(long_about = "Manage custom formula repositories (taps). Taps allow you to install \
                            packages from third-party repositories.")]
    Tap {
        #[command(subcommand)]
        command: Option<TapCommand>,
    },

    /// Manage lockfiles for reproducible environments
    Lock {
        #[command(subcommand)]
        command: Option<LockCommand>,
    },

    /// Manage background services
    Services {
        #[command(subcommand)]
        command: Option<ServicesCommand>,
    },

    /// Check system health and diagnose issues
    #[command(long_about = "Run diagnostic checks on your brewx installation. Reports issues \
                            with configuration, permissions, and installed packages.")]
    Doctor,

    /// Show brewx and system configuration
    Config,

    /// Generate shell completions for bash, zsh, or fish
    #[command(long_about = "Generate shell completion scripts. Add the output to your shell \
                            configuration file to enable tab completion.")]
    Completions {
        /// Shell to generate completions for
        shell: String,
    },

    /// Manage casks (GUI applications)
    #[command(long_about = "Install and manage macOS applications (casks) and Linux apps \
                            (AppImage, Flatpak).")]
    Cask {
        #[command(subcommand)]
        command: CaskCommand,
    },

    /// Manage Brewfile bundles
    #[command(long_about = "Install packages from a Brewfile or generate a Brewfile from \
                            installed packages.")]
    Bundle {
        #[command(subcommand)]
        command: Option<BundleCommand>,
    },

    /// Manage system snapshots
    Snapshot {
        #[command(subcommand)]
        command: SnapshotCommand,
    },

    /// Audit packages for known security vulnerabilities
    #[command(long_about = "Check installed packages against the vulnerability database. \
                            Reports CVEs and security advisories affecting your packages.")]
    Audit {
        /// Package to audit (all if omitted)
        package: Option<String>,
        /// Update vulnerability database first
        #[arg(long)]
        update: bool,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Manage offline mirrors
    Mirror {
        #[command(subcommand)]
        command: MirrorCommand,
    },

    /// Create and manage bottles (binary packages)
    Bottle {
        #[command(subcommand)]
        command: BottleCommand,
    },

    /// Create a new formula or cask from a URL
    Create {
        /// URL to create formula from
        url: String,
        /// Create a cask instead of formula
        #[arg(long)]
        cask: bool,
    },

    /// Test installed formulas
    Test {
        /// Package to test
        package: String,
    },

    /// Manage anonymous usage analytics
    Analytics {
        #[command(subcommand)]
        command: Option<AnalyticsCommand>,
    },

    /// Manage multiple installation prefixes
    Prefix {
        #[command(subcommand)]
        command: Option<PrefixCommand>,
    },
}

#[derive(Subcommand)]
enum TapCommand {
    /// Add a new tap
    Add {
        /// Tap name (user/repo) or URL
        name: String,
        /// Custom URL for the tap
        #[arg(long)]
        url: Option<String>,
    },
    /// Remove a tap
    Remove {
        /// Tap name to remove
        name: String,
    },
    /// List all taps
    List,
}

#[derive(Subcommand)]
enum LockCommand {
    /// Create a lockfile from installed packages
    Create,
    /// Install packages from lockfile
    Install,
    /// Show lockfile contents
    Show,
}

#[derive(Subcommand)]
enum ServicesCommand {
    /// List all services
    List,
    /// Start a service
    Start { name: String },
    /// Stop a service
    Stop { name: String },
    /// Restart a service
    Restart { name: String },
}

#[derive(Subcommand)]
enum CaskCommand {
    /// Install a cask
    Install { cask: String },
    /// Uninstall a cask
    Uninstall { cask: String },
    /// Search for casks
    Search { query: String },
    /// Show cask information
    Info { cask: String },
    /// List installed casks
    List,
    /// Show outdated casks
    Outdated,
    /// Upgrade casks
    Upgrade { casks: Vec<String> },
}

#[derive(Subcommand)]
enum BundleCommand {
    /// Install from Brewfile
    Install,
    /// Generate Brewfile from installed packages
    Dump,
    /// Check if Brewfile is satisfied
    Check,
    /// List Brewfile entries
    List,
    /// Remove packages not in Brewfile
    Cleanup,
}

#[derive(Subcommand)]
enum SnapshotCommand {
    /// Create a new snapshot
    Create { name: String },
    /// List all snapshots
    List,
    /// Restore a snapshot
    Restore { name: String },
    /// Delete a snapshot
    Delete { name: String },
}

#[derive(Subcommand)]
enum MirrorCommand {
    /// Create an offline mirror
    Create {
        /// Output directory
        dir: PathBuf,
        /// Packages to mirror
        packages: Vec<String>,
    },
    /// Serve a mirror via HTTP
    Serve {
        /// Mirror directory
        dir: PathBuf,
        /// Port to serve on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Show mirror information
    Info { dir: PathBuf },
    /// Verify mirror integrity
    Verify { dir: PathBuf },
}

#[derive(Subcommand)]
enum BottleCommand {
    /// Create a bottle from installed package
    Create { package: String },
    /// Upload a bottle
    Upload {
        package: String,
        #[arg(long)]
        url: String,
    },
}

#[derive(Subcommand)]
enum AnalyticsCommand {
    /// Enable analytics
    On,
    /// Disable analytics
    Off,
    /// Show analytics status
    Status,
}

#[derive(Subcommand)]
enum PrefixCommand {
    /// Create a new prefix
    Create { path: PathBuf },
    /// List all prefixes
    List,
    /// Show prefix information
    Info { path: Option<PathBuf> },
    /// Set default prefix
    Default { path: PathBuf },
    /// Remove a prefix
    Remove { path: PathBuf },
}

fn main() {
    // Only generate man pages when building for release or when explicitly requested
    if env::var("PROFILE").unwrap_or_default() != "release"
        && env::var("BREWX_GEN_MAN").is_err()
    {
        return;
    }

    let out_dir = match env::var_os("OUT_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => return,
    };

    let man_dir = out_dir.join("man");
    fs::create_dir_all(&man_dir).expect("Failed to create man directory");

    let cmd = Cli::command();

    // Generate main man page (brewx.1)
    let man = Man::new(cmd.clone());
    let mut buffer = Vec::new();
    man.render(&mut buffer).expect("Failed to render man page");
    fs::write(man_dir.join("brewx.1"), buffer).expect("Failed to write brewx.1");

    // Generate man pages for subcommands
    for subcommand in cmd.get_subcommands() {
        let name = subcommand.get_name();
        let man = Man::new(subcommand.clone());
        let mut buffer = Vec::new();
        man.render(&mut buffer)
            .expect(&format!("Failed to render man page for {}", name));
        fs::write(man_dir.join(format!("brewx-{}.1", name)), buffer)
            .expect(&format!("Failed to write brewx-{}.1", name));
    }

    // Tell cargo to rerun if CLI changes
    println!("cargo:rerun-if-changed=src/cli/mod.rs");
    println!("cargo:rerun-if-env-changed=BREWX_GEN_MAN");
}
