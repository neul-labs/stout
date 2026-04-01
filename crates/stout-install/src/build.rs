//! Build from source support
//!
//! This module provides functionality to build formulas from source
//! when pre-built bottles are not available for the current platform.

use crate::error::{BuildError, Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info};

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// Source archive URL
    pub source_url: String,
    /// Expected SHA256 hash
    pub sha256: String,
    /// Formula name
    pub name: String,
    /// Version
    pub version: String,
    /// Homebrew prefix (e.g., /opt/homebrew)
    pub prefix: PathBuf,
    /// Cellar path (e.g., /opt/homebrew/Cellar)
    pub cellar: PathBuf,
    /// Build dependencies to ensure are installed
    pub build_deps: Vec<String>,
    /// Number of parallel build jobs (default: auto-detect)
    pub jobs: Option<usize>,
    /// C compiler to use
    pub cc: Option<String>,
    /// C++ compiler to use
    pub cxx: Option<String>,
}

impl BuildConfig {
    /// Get the number of parallel jobs to use
    pub fn get_jobs(&self) -> usize {
        self.jobs.unwrap_or_else(num_cpus::get)
    }
}

/// Build result
#[derive(Debug)]
pub struct BuildResult {
    /// Path to the installed package
    pub install_path: PathBuf,
}

/// Source builder for formulas
pub struct SourceBuilder {
    config: BuildConfig,
    work_dir: PathBuf,
}

impl SourceBuilder {
    /// Create a new source builder
    pub fn new(config: BuildConfig, work_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            work_dir: work_dir.as_ref().to_path_buf(),
        }
    }

    /// Build the formula from source
    pub async fn build(&self) -> Result<BuildResult> {
        info!(
            "Building {} {} from source",
            self.config.name, self.config.version
        );

        // Create work directory
        std::fs::create_dir_all(&self.work_dir)?;

        // Download source
        let archive_path = self.download_source().await?;

        // Extract source
        let source_dir = self.extract_source(&archive_path)?;

        // Build
        let install_path = self.run_build(&source_dir)?;

        Ok(BuildResult { install_path })
    }

    /// Download the source archive
    async fn download_source(&self) -> Result<PathBuf> {
        use sha2::{Digest, Sha256};

        let archive_name = self
            .config
            .source_url
            .rsplit('/')
            .next()
            .unwrap_or("source.tar.gz");
        let archive_path = self.work_dir.join(archive_name);

        info!("Downloading source from {}", self.config.source_url);

        // Use reqwest to download
        let client = reqwest::Client::new();
        let response = client
            .get(&self.config.source_url)
            .send()
            .await
            .map_err(|e| {
                Error::Build(BuildError::DownloadFailed {
                    package: self.config.name.clone(),
                    reason: format!("Failed to download: {}", e),
                })
            })?;

        if !response.status().is_success() {
            return Err(Error::Build(BuildError::DownloadFailed {
                package: self.config.name.clone(),
                reason: format!("HTTP {}", response.status()),
            }));
        }

        let bytes = response.bytes().await.map_err(|e| {
            Error::Build(BuildError::DownloadFailed {
                package: self.config.name.clone(),
                reason: format!("Failed to read: {}", e),
            })
        })?;

        // Verify checksum (skip if not provided, e.g., for git-based sources)
        if !self.config.sha256.is_empty() {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = hex::encode(hasher.finalize());

            if hash != self.config.sha256 {
                return Err(Error::Build(BuildError::DownloadFailed {
                    package: self.config.name.clone(),
                    reason: format!(
                        "Checksum mismatch: expected {}, got {}",
                        self.config.sha256, hash
                    ),
                }));
            }
        } else {
            debug!("Skipping checksum verification (no sha256 provided)");
        }

        std::fs::write(&archive_path, &bytes)?;
        debug!("Downloaded and verified source archive");

        Ok(archive_path)
    }

    /// Extract the source archive
    fn extract_source(&self, archive_path: &Path) -> Result<PathBuf> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        info!("Extracting source archive");

        let file = std::fs::File::open(archive_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);

        // Extract to work directory
        archive.unpack(&self.work_dir)?;

        // Find the extracted directory (usually name-version)
        let expected_dir = format!("{}-{}", self.config.name, self.config.version);
        let source_dir = self.work_dir.join(&expected_dir);

        if source_dir.exists() {
            return Ok(source_dir);
        }

        // Try to find any directory that was created
        for entry in std::fs::read_dir(&self.work_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let name = entry.file_name();
                if name.to_string_lossy() != "." && name.to_string_lossy() != ".." {
                    return Ok(entry.path());
                }
            }
        }

        Err(Error::Build(BuildError::SourceDirectoryNotFound {
            package: self.config.name.clone(),
        }))
    }

    /// Run the build process
    fn run_build(&self, source_dir: &Path) -> Result<PathBuf> {
        let install_path = self
            .config
            .cellar
            .join(&self.config.name)
            .join(&self.config.version);

        info!("Building in {:?}", source_dir);
        info!("Install path: {:?}", install_path);

        // Create install directory
        std::fs::create_dir_all(&install_path)?;

        let params = BuildParams {
            name: &self.config.name,
            prefix: &self.config.prefix,
            jobs: self.config.get_jobs(),
            cc: self.config.cc.as_deref(),
            cxx: self.config.cxx.as_deref(),
        };

        detect_and_build(&params, source_dir, &install_path)?;

        Ok(install_path)
    }
}

/// Validate a compiler path for security
///
/// Ensures the path doesn't contain shell metacharacters or injection vectors.
/// While `Command::new()` doesn't invoke a shell, these paths may appear in
/// CMake `-D` flags or env vars that downstream tools interpolate.
fn validate_compiler_path(path: &str) -> Result<()> {
    // Check for empty path
    if path.trim().is_empty() {
        return Err(Error::Build(BuildError::CompilerValidationFailed {
            reason: "Compiler path cannot be empty".to_string(),
        }));
    }

    // Reject shell metacharacters and injection vectors
    let forbidden = [
        '!', '$', '`', '|', ';', '&', '(', ')', '{', '}', '\n', '\r', '\0',
    ];
    for ch in forbidden {
        if path.contains(ch) {
            return Err(Error::Build(BuildError::CompilerValidationFailed {
                reason: format!(
                    "Invalid compiler path '{}': contains forbidden character '{}'",
                    path, ch
                ),
            }));
        }
    }

    // Check for path traversal
    if path.contains("..") {
        return Err(Error::Build(BuildError::CompilerValidationFailed {
            reason: format!("Invalid compiler path '{}': contains path traversal", path),
        }));
    }

    Ok(())
}

/// Check if build from source is available for a formula
pub fn can_build_from_source(source_url: &Option<String>) -> bool {
    source_url.is_some()
}

/// Check if a file is executable
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = path.metadata() {
        let permissions = metadata.permissions();
        permissions.mode() & 0o111 != 0
    } else {
        false
    }
}

// ============================================================================
// Shared build system implementations
// ============================================================================

/// Common parameters needed by all build system implementations.
struct BuildParams<'a> {
    name: &'a str,
    prefix: &'a Path,
    jobs: usize,
    cc: Option<&'a str>,
    cxx: Option<&'a str>,
}

/// Apply validated CC/CXX environment variables to a command.
fn apply_compiler_env(cmd: &mut Command, params: &BuildParams) -> Result<()> {
    if let Some(cc) = params.cc {
        validate_compiler_path(cc)?;
        cmd.env("CC", cc);
    }
    if let Some(cxx) = params.cxx {
        validate_compiler_path(cxx)?;
        cmd.env("CXX", cxx);
    }
    Ok(())
}

/// Detect the build system and run the appropriate build steps.
fn detect_and_build(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    if source_dir.join("CMakeLists.txt").exists() {
        build_cmake(params, source_dir, install_path)
    } else if source_dir.join("configure").exists() {
        build_autotools(params, source_dir, install_path)
    } else if source_dir.join("Makefile").exists() {
        build_make(params, source_dir, install_path)
    } else if source_dir.join("meson.build").exists() {
        build_meson(params, source_dir, install_path)
    } else if source_dir.join("Cargo.toml").exists() {
        build_cargo(params, source_dir, install_path)
    } else {
        Err(Error::Build(BuildError::unknown_build_system(params.name)))
    }
}

/// Run autogen.sh to generate a configure script.
fn run_autogen(params: &BuildParams, source_dir: &Path) -> Result<()> {
    info!("Running autogen.sh");

    let status = Command::new("./autogen.sh")
        .current_dir(source_dir)
        .status()?;

    if !status.success() {
        return Err(Error::Build(BuildError::ConfigureFailed {
            package: params.name.to_string(),
        }));
    }

    Ok(())
}

/// Build using autotools (configure/make/make install).
fn build_autotools(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    info!("Using autotools build system");

    let mut configure_cmd = Command::new("./configure");
    configure_cmd
        .arg(format!("--prefix={}", install_path.display()))
        .current_dir(source_dir)
        .env("HOMEBREW_PREFIX", params.prefix);
    apply_compiler_env(&mut configure_cmd, params)?;

    if !configure_cmd.status()?.success() {
        return Err(Error::Build(BuildError::configure_failed(params.name)));
    }

    let mut make_cmd = Command::new("make");
    make_cmd
        .arg("-j")
        .arg(params.jobs.to_string())
        .current_dir(source_dir);
    apply_compiler_env(&mut make_cmd, params)?;

    if !make_cmd.status()?.success() {
        return Err(Error::Build(BuildError::make_failed(params.name)));
    }

    let install_status = Command::new("make")
        .arg("install")
        .arg("--")
        .current_dir(source_dir)
        .status()?;

    if !install_status.success() {
        return Err(Error::Build(BuildError::make_install_failed(params.name)));
    }

    Ok(())
}

/// Build using CMake.
fn build_cmake(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    info!("Using CMake build system");

    let build_dir = source_dir.join("build");
    std::fs::create_dir_all(&build_dir)?;

    let mut cmake_cmd = Command::new("cmake");
    cmake_cmd
        .arg("..")
        .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_path.display()))
        .arg("-DCMAKE_BUILD_TYPE=Release")
        .current_dir(&build_dir);

    if let Some(cc) = params.cc {
        validate_compiler_path(cc)?;
        cmake_cmd.arg(format!("-DCMAKE_C_COMPILER={}", cc));
    }
    if let Some(cxx) = params.cxx {
        validate_compiler_path(cxx)?;
        cmake_cmd.arg(format!("-DCMAKE_CXX_COMPILER={}", cxx));
    }

    if !cmake_cmd.status()?.success() {
        return Err(Error::Build(BuildError::CmakeConfigureFailed {
            package: params.name.to_string(),
        }));
    }

    let build_status = Command::new("cmake")
        .arg("--build")
        .arg(".")
        .arg("-j")
        .arg(params.jobs.to_string())
        .current_dir(&build_dir)
        .status()?;

    if !build_status.success() {
        return Err(Error::Build(BuildError::CmakeBuildFailed {
            package: params.name.to_string(),
        }));
    }

    let install_status = Command::new("cmake")
        .arg("--install")
        .arg(".")
        .current_dir(&build_dir)
        .status()?;

    if !install_status.success() {
        return Err(Error::Build(BuildError::CmakeInstallFailed {
            package: params.name.to_string(),
        }));
    }

    Ok(())
}

/// Build using plain Makefile.
fn build_make(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    info!("Using Makefile build system");

    let mut make_cmd = Command::new("make");
    make_cmd
        .arg("-j")
        .arg(params.jobs.to_string())
        .current_dir(source_dir)
        .env("PREFIX", install_path);
    apply_compiler_env(&mut make_cmd, params)?;

    if !make_cmd.status()?.success() {
        return Err(Error::Build(BuildError::make_failed(params.name)));
    }

    let install_status = Command::new("make")
        .arg("install")
        .arg(format!("PREFIX={}", install_path.display()))
        .current_dir(source_dir)
        .status()?;

    if !install_status.success() {
        return Err(Error::Build(BuildError::make_install_failed(params.name)));
    }

    Ok(())
}

/// Build using Meson.
fn build_meson(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    info!("Using Meson build system");

    let build_dir = source_dir.join("build");

    let mut setup_cmd = Command::new("meson");
    setup_cmd
        .arg("setup")
        .arg(&build_dir)
        .arg(format!("--prefix={}", install_path.display()))
        .current_dir(source_dir);
    apply_compiler_env(&mut setup_cmd, params)?;

    if !setup_cmd.status()?.success() {
        return Err(Error::Build(BuildError::MesonConfigureFailed {
            package: params.name.to_string(),
        }));
    }

    let compile_status = Command::new("meson")
        .arg("compile")
        .arg("-C")
        .arg(&build_dir)
        .arg("-j")
        .arg(params.jobs.to_string())
        .status()?;

    if !compile_status.success() {
        return Err(Error::Build(BuildError::MesonCompileFailed {
            package: params.name.to_string(),
        }));
    }

    let install_status = Command::new("meson")
        .arg("install")
        .arg("-C")
        .arg(&build_dir)
        .status()?;

    if !install_status.success() {
        return Err(Error::Build(BuildError::MesonInstallFailed {
            package: params.name.to_string(),
        }));
    }

    Ok(())
}

/// Build using Cargo (Rust).
fn build_cargo(params: &BuildParams, source_dir: &Path, install_path: &Path) -> Result<()> {
    info!("Using Cargo build system");

    let build_status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("-j")
        .arg(params.jobs.to_string())
        .current_dir(source_dir)
        .status()?;

    if !build_status.success() {
        return Err(Error::Build(BuildError::CargoBuildFailed {
            package: params.name.to_string(),
        }));
    }

    let bin_dir = install_path.join("bin");
    std::fs::create_dir_all(&bin_dir)?;

    let release_dir = source_dir.join("target/release");
    if release_dir.exists() {
        for entry in std::fs::read_dir(&release_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && is_executable(&path) {
                let file_name = path.file_name().unwrap();
                let name = file_name.to_string_lossy();
                if !name.contains('.') && !name.starts_with("lib") {
                    let dest = bin_dir.join(file_name);
                    std::fs::copy(&path, &dest)?;
                    debug!("Installed binary: {:?}", dest);
                }
            }
        }
    }

    Ok(())
}

// ============================================================================
// HEAD Build Support
// ============================================================================

/// Configuration for building from HEAD (git)
#[derive(Debug, Clone)]
pub struct HeadBuildConfig {
    /// Git repository URL
    pub git_url: String,
    /// Branch to clone (default: "master")
    pub branch: String,
    /// Formula name
    pub name: String,
    /// Homebrew prefix
    pub prefix: PathBuf,
    /// Cellar path
    pub cellar: PathBuf,
    /// Number of parallel build jobs
    pub jobs: Option<usize>,
    /// C compiler
    pub cc: Option<String>,
    /// C++ compiler
    pub cxx: Option<String>,
}

impl HeadBuildConfig {
    /// Get the number of parallel jobs to use
    pub fn get_jobs(&self) -> usize {
        self.jobs.unwrap_or_else(num_cpus::get)
    }
}

/// Result of a HEAD build
#[derive(Debug)]
pub struct HeadBuildResult {
    /// Path to installed package
    pub install_path: PathBuf,
    /// Full commit SHA
    pub commit_sha: String,
    /// Short SHA (7 chars)
    pub short_sha: String,
}

/// Builder for HEAD (git) installations
pub struct HeadBuilder {
    config: HeadBuildConfig,
    work_dir: PathBuf,
}

impl HeadBuilder {
    /// Create a new HEAD builder
    pub fn new(config: HeadBuildConfig, work_dir: impl AsRef<Path>) -> Self {
        Self {
            config,
            work_dir: work_dir.as_ref().to_path_buf(),
        }
    }

    /// Build from HEAD git repository
    pub async fn build(&self) -> Result<HeadBuildResult> {
        info!(
            "Building {} from HEAD ({})",
            self.config.name, self.config.git_url
        );

        // Create work directory
        std::fs::create_dir_all(&self.work_dir)?;

        // Clone repository
        let repo_dir = self.clone_repository()?;

        // Get commit SHA
        let (full_sha, short_sha) = self.get_commit_sha(&repo_dir)?;
        info!("HEAD commit: {} ({})", short_sha, full_sha);

        // Build using detected build system
        let install_path = self.run_build(&repo_dir, &short_sha)?;

        Ok(HeadBuildResult {
            install_path,
            commit_sha: full_sha,
            short_sha,
        })
    }

    /// Clone the git repository
    fn clone_repository(&self) -> Result<PathBuf> {
        let repo_dir = self.work_dir.join(&self.config.name);

        // Remove existing directory if present
        if repo_dir.exists() {
            debug!("Removing existing repository directory");
            std::fs::remove_dir_all(&repo_dir)?;
        }

        info!("Cloning {}...", self.config.git_url);

        // Clone with depth 1 for efficiency (we just need the latest)
        let mut args = vec!["clone", "--depth", "1"];

        // Add branch if not default
        if !self.config.branch.is_empty()
            && self.config.branch != "master"
            && self.config.branch != "main"
        {
            args.extend_from_slice(&["--branch", &self.config.branch]);
        }

        args.extend_from_slice(&[&self.config.git_url, repo_dir.to_str().unwrap()]);

        let status = Command::new("git").args(&args).status().map_err(|e| {
            Error::Build(BuildError::GitCloneFailed {
                package: self.config.name.clone(),
                reason: e.to_string(),
            })
        })?;

        if !status.success() {
            return Err(Error::Build(BuildError::GitCloneFailed {
                package: self.config.name.clone(),
                reason: "git clone returned non-zero exit code".to_string(),
            }));
        }

        debug!("Repository cloned to {:?}", repo_dir);
        Ok(repo_dir)
    }

    /// Get the current commit SHA from the cloned repository
    fn get_commit_sha(&self, repo_dir: &Path) -> Result<(String, String)> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(repo_dir)
            .output()
            .map_err(|e| {
                Error::Build(BuildError::GitFailed {
                    package: self.config.name.clone(),
                    reason: e.to_string(),
                })
            })?;

        if !output.status.success() {
            return Err(Error::Build(BuildError::GitFailed {
                package: self.config.name.clone(),
                reason: "Failed to get commit SHA".to_string(),
            }));
        }

        let full_sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let short_sha: String = full_sha.chars().take(7).collect();

        Ok((full_sha, short_sha))
    }

    /// Build using detected build system
    fn run_build(&self, source_dir: &Path, short_sha: &str) -> Result<PathBuf> {
        let install_path = self
            .config
            .cellar
            .join(&self.config.name)
            .join(format!("HEAD-{}", short_sha));

        info!("Building in {:?}", source_dir);
        info!("Install path: {:?}", install_path);

        // Create install directory
        std::fs::create_dir_all(&install_path)?;

        let params = BuildParams {
            name: &self.config.name,
            prefix: &self.config.prefix,
            jobs: self.config.get_jobs(),
            cc: self.config.cc.as_deref(),
            cxx: self.config.cxx.as_deref(),
        };

        // HEAD builds also check for autogen.sh before configure
        if source_dir.join("autogen.sh").exists() && !source_dir.join("configure").exists() {
            run_autogen(&params, source_dir)?;
        }

        detect_and_build(&params, source_dir, &install_path)?;

        Ok(install_path)
    }
}
