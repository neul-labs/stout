//! INSTALL_RECEIPT.json handling

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// INSTALL_RECEIPT.json structure (Homebrew compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReceipt {
    pub homebrew_version: String,
    pub installed_as_dependency: bool,
    pub installed_on_request: bool,
    pub install_time: u64,
    pub source: ReceiptSource,
    #[serde(default)]
    pub runtime_dependencies: Vec<RuntimeDependency>,
    #[serde(default)]
    pub poured_from_bottle: bool,
    #[serde(default)]
    pub built_as_bottle: bool,
    #[serde(default)]
    pub changed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptSource {
    pub tap: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeDependency {
    pub full_name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<u32>,
}

impl InstallReceipt {
    /// Create a new receipt for a bottle installation
    pub fn new_bottle(
        tap: &str,
        on_request: bool,
        dependencies: Vec<RuntimeDependency>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            homebrew_version: "4.0.0".to_string(), // Compatible version
            installed_as_dependency: !on_request,
            installed_on_request: on_request,
            install_time: now,
            source: ReceiptSource {
                tap: tap.to_string(),
                path: None,
            },
            runtime_dependencies: dependencies,
            poured_from_bottle: true,
            built_as_bottle: true,
            changed_files: Vec::new(),
        }
    }

    /// Create a new receipt for a source-built installation
    pub fn new_source(
        tap: &str,
        on_request: bool,
        dependencies: Vec<RuntimeDependency>,
    ) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            homebrew_version: "4.0.0".to_string(),
            installed_as_dependency: !on_request,
            installed_on_request: on_request,
            install_time: now,
            source: ReceiptSource {
                tap: tap.to_string(),
                path: None,
            },
            runtime_dependencies: dependencies,
            poured_from_bottle: false, // Built from source
            built_as_bottle: false,
            changed_files: Vec::new(),
        }
    }
}

/// Write an INSTALL_RECEIPT.json file
pub fn write_receipt(install_path: impl AsRef<Path>, receipt: &InstallReceipt) -> Result<()> {
    let path = install_path.as_ref().join("INSTALL_RECEIPT.json");
    let json = serde_json::to_string_pretty(receipt)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Read an existing INSTALL_RECEIPT.json
#[allow(dead_code)]
pub fn read_receipt(install_path: impl AsRef<Path>) -> Result<InstallReceipt> {
    let path = install_path.as_ref().join("INSTALL_RECEIPT.json");
    let json = std::fs::read_to_string(&path)?;
    let receipt: InstallReceipt = serde_json::from_str(&json)?;
    Ok(receipt)
}
