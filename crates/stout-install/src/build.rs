//! Build from source support
//!
//! This module provides functionality to build formulas from source
//! when pre-built bottles are not available for the current platform.

use crate::error::{BuildError, Error, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

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
        info!("Building {} {} from source", self.config.name, self.config.version);

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

        let archive_name = self.config.source_url
            .rsplit('/')
            .next()
            .unwrap_or("source.tar.gz");
        let archive_path = self.work_dir.join(archive_name);

        info!("Downloading source from {}", self.config.source_url);

        // Use reqwest to download
        let client = reqwest::Client::new();
        let response = client.get(&self.config.source_url)
            .send()
            .await
            .map_err(|e| Error::Build(format!("Failed to download source: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Build(format!(
                "Failed to download source: HTTP {}",
                response.status()
            )));
        }

        let bytes = response.bytes()
            .await
            .map_err(|e| Error::Build(format!("Failed to read source: {}", e)))?;

        // Verify checksum
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let hash = format!("{:x}", hasher.finalize());

        if hash != self.config.sha256 {
            return Err(Error::Build(format!(
                "Checksum mismatch: expected {}, got {}",
                self.config.sha256, hash
            )));
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

        Err(Error::Build("Could not find extracted source directory".to_string()))
    }

    /// Run the build process
    fn run_build(&self, source_dir: &Path) -> Result<PathBuf> {
        let install_path = self.config.cellar
            .join(&self.config.name)
            .join(&self.config.version);

        info!("Building in {:?}", source_dir);
        info!("Install path: {:?}", install_path);

        // Create install directory
        std::fs::create_dir_all(&install_path)?;

        // Detect build system and run appropriate commands
        if source_dir.join("CMakeLists.txt").exists() {
            self.build_cmake(source_dir, &install_path)?;
        } else if source_dir.join("configure").exists() {
            self.build_autotools(source_dir, &install_path)?;
        } else if source_dir.join("Makefile").exists() {
            self.build_make(source_dir, &install_path)?;
        } else if source_dir.join("meson.build").exists() {
            self.build_meson(source_dir, &install_path)?;
        } else if source_dir.join("Cargo.toml").exists() {
            self.build_cargo(source_dir, &install_path)?;
        } else {
            return Err(Error::Build(BuildError::unknown_build_system(&self.config.name)));
        }

        Ok(install_path)
    }

    /// Build using autotools (configure/make/make install)
    fn build_autotools(&self, source_dir: &Path, install_path: &Path) -> Result<()> {
        info!("Using autotools build system");

        let mut configure_cmd = Command::new("./configure");
        configure_cmd
            .arg(format!("--prefix={}", install_path.display()))
            .current_dir(source_dir)
            .env("HOMEBREW_PREFIX", &self.config.prefix);

        // Set compilers if specified
        if let Some(cc) = &self.config.cc {
            configure_cmd.env("CC", cc);
        }
        if let Some(cxx) = &self.config.cxx {
            configure_cmd.env("CXX", cxx);
        }

        let configure_status = configure_cmd.status()?;

        if !configure_status.success() {
            return Err(Error::Build(BuildError::configure_failed(&self.config.name)));
        }

        // Make
        let mut make_cmd = Command::new("make");
        make_cmd
            .arg("-j")
            .arg(self.config.get_jobs().to_string())
            .current_dir(source_dir);

        // Set compilers for make too
        if let Some(cc) = &self.config.cc {
            make_cmd.env("CC", cc);
        }
        if let Some(cxx) = &self.config.cxx {
            make_cmd.env("CXX", cxx);
        }

        let make_status = make_cmd.status()?;

        if !make_status.success() {
            return Err(Error::Build(BuildError::make_failed(&self.config.name)));
        }

        // Make install
        // Use -- to prevent any argument injection - everything after -- is treated as a target
        let install_status = Command::new("make")
            .arg("install")
            .arg("--")
            .current_dir(source_dir)
            .status()?;

        if !install_status.success() {
            return Err(Error::Build(BuildError::make_install_failed(&self.config.name)));
        }

        Ok(())
    }

    /// Build using CMake
    fn build_cmake(&self, source_dir: &Path, install_path: &Path) -> Result<()> {
        info!("Using CMake build system");

        let build_dir = source_dir.join("build");
        std::fs::create_dir_all(&build_dir)?;

        // Configure
        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("..")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", install_path.display()))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .current_dir(&build_dir);

        // Set compilers if specified
        if let Some(cc) = &self.config.cc {
            cmake_cmd.arg(format!("-DCMAKE_C_COMPILER={}", cc));
        }
        if let Some(cxx) = &self.config.cxx {
            cmake_cmd.arg(format!("-DCMAKE_CXX_COMPILER={}", cxx));
        }

        let cmake_status = cmake_cmd.status()?;

        if !cmake_status.success() {
            return Err(Error::Build(BuildError::CmakeConfigureFailed {
                package: self.config.name.clone()
            }));
        }

        // Build
        let build_status = Command::new("cmake")
            .arg("--build")
            .arg(".")
            .arg("-j")
            .arg(self.config.get_jobs().to_string())
            .current_dir(&build_dir)
            .status()?;

        if !build_status.success() {
            return Err(Error::Build(BuildError::CmakeBuildFailed {
                package: self.config.name.clone()
            }));
        }

        // Install
        let install_status = Command::new("cmake")
            .arg("--install")
            .arg(".")
            .current_dir(&build_dir)
            .status()?;

        if !install_status.success() {
            return Err(Error::Build(BuildError::CmakeInstallFailed {
                package: self.config.name.clone()
            }));
        }

        Ok(())
    }

    /// Build using plain Makefile
    fn build_make(&self, source_dir: &Path, install_path: &Path) -> Result<()> {
        info!("Using Makefile build system");

        // Make
        let mut make_cmd = Command::new("make");
        make_cmd
            .arg("-j")
            .arg(self.config.get_jobs().to_string())
            .current_dir(source_dir)
            .env("PREFIX", install_path);

        // Set compilers if specified
        if let Some(cc) = &self.config.cc {
            make_cmd.env("CC", cc);
        }
        if let Some(cxx) = &self.config.cxx {
            make_cmd.env("CXX", cxx);
        }

        let make_status = make_cmd.status()?;

        if !make_status.success() {
            return Err(Error::Build(BuildError::make_failed(&self.config.name)));
        }

        // Make install
        let install_status = Command::new("make")
            .arg("install")
            .arg(format!("PREFIX={}", install_path.display()))
            .current_dir(source_dir)
            .status()?;

        if !install_status.success() {
            return Err(Error::Build(BuildError::make_install_failed(&self.config.name)));
        }

        Ok(())
    }

    /// Build using Meson
    fn build_meson(&self, source_dir: &Path, install_path: &Path) -> Result<()> {
        info!("Using Meson build system");

        let build_dir = source_dir.join("build");

        // Setup with compiler options
        let mut setup_cmd = Command::new("meson");
        setup_cmd
            .arg("setup")
            .arg(&build_dir)
            .arg(format!("--prefix={}", install_path.display()))
            .current_dir(source_dir);

        // Set compilers if specified (meson uses CC/CXX env vars)
        if let Some(cc) = &self.config.cc {
            setup_cmd.env("CC", cc);
        }
        if let Some(cxx) = &self.config.cxx {
            setup_cmd.env("CXX", cxx);
        }

        let setup_status = setup_cmd.status()?;

        if !setup_status.success() {
            return Err(Error::Build("meson setup failed".to_string()));
        }

        // Compile with parallel jobs
        let compile_status = Command::new("meson")
            .arg("compile")
            .arg("-C")
            .arg(&build_dir)
            .arg("-j")
            .arg(self.config.get_jobs().to_string())
            .status()?;

        if !compile_status.success() {
            return Err(Error::Build("meson compile failed".to_string()));
        }

        // Install
        let install_status = Command::new("meson")
            .arg("install")
            .arg("-C")
            .arg(&build_dir)
            .status()?;

        if !install_status.success() {
            return Err(Error::Build("meson install failed".to_string()));
        }

        Ok(())
    }

    /// Build using Cargo (Rust)
    fn build_cargo(&self, source_dir: &Path, install_path: &Path) -> Result<()> {
        info!("Using Cargo build system");

        // Build release with configurable jobs
        let build_status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .arg("-j")
            .arg(self.config.get_jobs().to_string())
            .current_dir(source_dir)
            .status()?;

        if !build_status.success() {
            return Err(Error::Build("cargo build failed".to_string()));
        }

        // Install binaries
        let bin_dir = install_path.join("bin");
        std::fs::create_dir_all(&bin_dir)?;

        let release_dir = source_dir.join("target/release");
        if release_dir.exists() {
            for entry in std::fs::read_dir(&release_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() && is_executable(&path) {
                    let file_name = path.file_name().unwrap();
                    // Skip common non-binary files
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

/// Check if build from source is available for a formula
pub fn can_build_from_source(source_url: &Option<String>) -> bool {
    source_url.is_some()
}
