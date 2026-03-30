//! Error types for stout-install

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Link failed: {0}")]
    LinkFailed(String),

    #[error("Invalid bottle format: {0}")]
    InvalidBottle(String),

    #[error("Build failed: {0}")]
    Build(#[from] BuildError),

    #[error("Bottle creation failed: {0}")]
    Bottle(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("{0}")]
    Other(String),
}

/// Specific build errors with context
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("configure script failed for {package}")]
    ConfigureFailed { package: String },

    #[error("configure script not found for {package}")]
    ConfigureNotFound { package: String },

    #[error("make failed for {package}")]
    MakeFailed { package: String },

    #[error("make install failed for {package}")]
    MakeInstallFailed { package: String },

    #[error("cmake configure failed for {package}")]
    CmakeConfigureFailed { package: String },

    #[error("cmake build failed for {package}")]
    CmakeBuildFailed { package: String },

    #[error("cmake install failed for {package}")]
    CmakeInstallFailed { package: String },

    #[error("meson configure failed for {package}")]
    MesonConfigureFailed { package: String },

    #[error("meson build failed for {package}")]
    MesonBuildFailed { package: String },

    #[error("cargo build failed for {package}")]
    CargoBuildFailed { package: String },

    #[error("meson compile failed for {package}")]
    MesonCompileFailed { package: String },

    #[error("meson install failed for {package}")]
    MesonInstallFailed { package: String },

    #[error("unknown build system for {package}")]
    UnknownBuildSystem { package: String },

    #[error("download failed for {package}: {reason}")]
    DownloadFailed { package: String, reason: String },

    #[error("extraction failed for {package}: {reason}")]
    ExtractionFailed { package: String, reason: String },

    #[error("source directory not found for {package}")]
    SourceDirectoryNotFound { package: String },

    #[error("compiler validation failed: {reason}")]
    CompilerValidationFailed { reason: String },

    #[error("git clone failed for {package}: {reason}")]
    GitCloneFailed { package: String, reason: String },

    #[error("git operation failed for {package}: {reason}")]
    GitFailed { package: String, reason: String },

    #[error("no HEAD URL available for {package}")]
    HeadUrlMissing { package: String },
}

impl BuildError {
    /// Create a ConfigureFailed error
    pub fn configure_failed(package: &str) -> Self {
        BuildError::ConfigureFailed {
            package: package.to_string(),
        }
    }

    /// Create a MakeFailed error
    pub fn make_failed(package: &str) -> Self {
        BuildError::MakeFailed {
            package: package.to_string(),
        }
    }

    /// Create a MakeInstallFailed error
    pub fn make_install_failed(package: &str) -> Self {
        BuildError::MakeInstallFailed {
            package: package.to_string(),
        }
    }

    /// Create a UnknownBuildSystem error
    pub fn unknown_build_system(package: &str) -> Self {
        BuildError::UnknownBuildSystem {
            package: package.to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
