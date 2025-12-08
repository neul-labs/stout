//! Installed cask state tracking

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Information about an installed cask
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledCask {
    pub version: String,
    pub installed_at: String,
    pub artifact_path: PathBuf,
    #[serde(default)]
    pub auto_updates: bool,
    #[serde(default)]
    pub artifacts: Vec<InstalledArtifact>,
}

/// An artifact that was installed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledArtifact {
    pub artifact_type: String,
    pub source: String,
    pub installed_path: PathBuf,
}

/// Collection of installed casks
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct InstalledCasks {
    #[serde(default)]
    pub casks: HashMap<String, InstalledCask>,
}

impl InstalledCasks {
    /// Load installed casks from file
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let contents = std::fs::read_to_string(path)
                .map_err(|e| Error::State(format!("Failed to read casks file: {}", e)))?;
            let casks: InstalledCasks = serde_json::from_str(&contents)
                .map_err(|e| Error::State(format!("Failed to parse casks file: {}", e)))?;
            Ok(casks)
        } else {
            Ok(Self::default())
        }
    }

    /// Save installed casks to file
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| Error::State(format!("Failed to create directory: {}", e)))?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| Error::State(format!("Failed to serialize casks: {}", e)))?;
        std::fs::write(path, contents)
            .map_err(|e| Error::State(format!("Failed to write casks file: {}", e)))?;
        Ok(())
    }

    /// Add an installed cask
    pub fn add(&mut self, token: &str, cask: InstalledCask) {
        self.casks.insert(token.to_string(), cask);
    }

    /// Remove an installed cask
    pub fn remove(&mut self, token: &str) -> Option<InstalledCask> {
        self.casks.remove(token)
    }

    /// Get an installed cask
    pub fn get(&self, token: &str) -> Option<&InstalledCask> {
        self.casks.get(token)
    }

    /// Check if a cask is installed
    pub fn is_installed(&self, token: &str) -> bool {
        self.casks.contains_key(token)
    }

    /// Get all installed cask tokens
    pub fn tokens(&self) -> impl Iterator<Item = &String> {
        self.casks.keys()
    }

    /// Get count of installed casks
    pub fn count(&self) -> usize {
        self.casks.len()
    }

    /// Iterate over all installed casks
    pub fn iter(&self) -> impl Iterator<Item = (&String, &InstalledCask)> {
        self.casks.iter()
    }
}

/// Simple timestamp
pub fn now_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    let years = 1970 + (days_since_epoch / 365);
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30).min(11) + 1;
    let day = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        years, month, day, hours, minutes, seconds
    )
}
