//! Cask types and structures for macOS applications

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Basic cask info stored in the SQLite index (fast queries)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaskInfo {
    pub token: String,
    pub name: Option<String>,
    pub version: String,
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub tap: String,
    pub deprecated: bool,
    pub disabled: bool,
    pub artifact_type: Option<String>,
    pub json_hash: Option<String>,
}

/// Full cask data from individual JSON files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cask {
    pub token: String,

    #[serde(default)]
    pub name: Vec<String>,

    pub version: String,

    pub desc: Option<String>,
    pub homepage: Option<String>,

    #[serde(default)]
    pub tap: String,

    pub url: Option<String>,

    #[serde(default)]
    pub sha256: CaskSha256,

    #[serde(default)]
    pub artifacts: Vec<CaskArtifact>,

    #[serde(default)]
    pub depends_on: CaskDependsOn,

    pub caveats: Option<String>,

    #[serde(default)]
    pub auto_updates: bool,

    #[serde(default)]
    pub deprecated: bool,

    #[serde(default)]
    pub disabled: bool,

    /// Container type (e.g., "dmg", "zip", "pkg")
    pub container: Option<ContainerSpec>,

    /// URL specs for different architectures
    #[serde(default)]
    pub url_specs: HashMap<String, UrlSpec>,
}

/// SHA256 can be a string or "no_check"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CaskSha256 {
    Hash(String),
    #[default]
    NoCheck,
}

impl CaskSha256 {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            CaskSha256::Hash(s) => Some(s),
            CaskSha256::NoCheck => None,
        }
    }
}

/// Cask artifact - what gets installed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CaskArtifact {
    App(AppArtifact),
    Pkg(PkgArtifact),
    Binary(BinaryArtifact),
    Zap(ZapArtifact),
    Uninstall(UninstallArtifact),
    Other(serde_json::Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppArtifact {
    pub app: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PkgArtifact {
    pub pkg: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryArtifact {
    pub binary: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapArtifact {
    pub zap: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallArtifact {
    pub uninstall: Vec<serde_json::Value>,
}

/// Container specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerSpec {
    #[serde(rename = "type")]
    pub container_type: Option<String>,
    pub nested: Option<String>,
}

/// URL specification for architecture-specific downloads
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlSpec {
    pub url: String,
    pub sha256: Option<String>,
}

/// Cask dependencies
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CaskDependsOn {
    #[serde(default)]
    pub formula: Vec<String>,

    #[serde(default)]
    pub cask: Vec<String>,

    #[serde(default)]
    pub macos: Option<MacOsRequirement>,
}

/// macOS version requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MacOsRequirement {
    Version(String),
    Versions(Vec<String>),
    Comparison { op: String, version: String },
}

impl Cask {
    /// Get the display name (first name or token)
    pub fn display_name(&self) -> &str {
        self.name.first().map(|s| s.as_str()).unwrap_or(&self.token)
    }

    /// Get the primary artifact type
    pub fn primary_artifact_type(&self) -> &'static str {
        for artifact in &self.artifacts {
            match artifact {
                CaskArtifact::App(_) => return "app",
                CaskArtifact::Pkg(_) => return "pkg",
                CaskArtifact::Binary(_) => return "binary",
                _ => continue,
            }
        }
        "unknown"
    }

    /// Get all app artifacts
    pub fn apps(&self) -> Vec<&str> {
        self.artifacts
            .iter()
            .filter_map(|a| match a {
                CaskArtifact::App(app) => {
                    Some(app.app.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                }
                _ => None,
            })
            .flatten()
            .collect()
    }

    /// Get the download URL for the current architecture
    pub fn download_url(&self) -> Option<&str> {
        // Check architecture-specific URLs first
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "intel"
        };

        if let Some(spec) = self.url_specs.get(arch) {
            return Some(&spec.url);
        }

        self.url.as_deref()
    }

    /// Get formula dependencies
    pub fn formula_deps(&self) -> &[String] {
        &self.depends_on.formula
    }

    /// Get cask dependencies
    pub fn cask_deps(&self) -> &[String] {
        &self.depends_on.cask
    }
}
