//! Snapshot management for stout
//!
//! Snapshots capture the current state of installed packages for quick
//! backup and restoration.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// A snapshot of installed packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Snapshot name
    pub name: String,

    /// Optional description
    #[serde(default)]
    pub description: Option<String>,

    /// When the snapshot was created
    pub created_at: String,

    /// stout version that created this snapshot
    pub stout_version: String,

    /// Installed formulas
    #[serde(default)]
    pub formulas: Vec<FormulaSnapshot>,

    /// Installed casks
    #[serde(default)]
    pub casks: Vec<CaskSnapshot>,

    /// Pinned packages
    #[serde(default)]
    pub pinned: Vec<String>,

    /// Active taps
    #[serde(default)]
    pub taps: Vec<String>,
}

/// Formula in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaSnapshot {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub revision: u32,
    /// Whether this was explicitly installed (vs dependency)
    #[serde(default)]
    pub requested: bool,
}

/// Cask in a snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaskSnapshot {
    pub token: String,
    pub version: String,
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(name: &str, description: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            description: description.map(|s| s.to_string()),
            created_at: current_timestamp(),
            stout_version: env!("CARGO_PKG_VERSION").to_string(),
            formulas: Vec::new(),
            casks: Vec::new(),
            pinned: Vec::new(),
            taps: Vec::new(),
        }
    }

    /// Add a formula to the snapshot
    pub fn add_formula(&mut self, name: &str, version: &str, revision: u32, requested: bool) {
        self.formulas.push(FormulaSnapshot {
            name: name.to_string(),
            version: version.to_string(),
            revision,
            requested,
        });
    }

    /// Add a cask to the snapshot
    pub fn add_cask(&mut self, token: &str, version: &str) {
        self.casks.push(CaskSnapshot {
            token: token.to_string(),
            version: version.to_string(),
        });
    }

    /// Get formula count
    pub fn formula_count(&self) -> usize {
        self.formulas.len()
    }

    /// Get cask count
    pub fn cask_count(&self) -> usize {
        self.casks.len()
    }

    /// Get requested formula names
    pub fn requested_formulas(&self) -> Vec<&str> {
        self.formulas
            .iter()
            .filter(|f| f.requested)
            .map(|f| f.name.as_str())
            .collect()
    }
}

/// Manages snapshots on disk
pub struct SnapshotManager {
    snapshots_dir: PathBuf,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(stout_dir: &Path) -> Self {
        Self {
            snapshots_dir: stout_dir.join("snapshots"),
        }
    }

    /// Ensure snapshots directory exists
    fn ensure_dir(&self) -> Result<()> {
        std::fs::create_dir_all(&self.snapshots_dir)?;
        Ok(())
    }

    /// Get path for a snapshot file
    fn snapshot_path(&self, name: &str) -> PathBuf {
        self.snapshots_dir.join(format!("{}.json", name))
    }

    /// Save a snapshot to disk
    pub fn save(&self, snapshot: &Snapshot) -> Result<PathBuf> {
        self.ensure_dir()?;

        let path = self.snapshot_path(&snapshot.name);
        let json = serde_json::to_string_pretty(snapshot)?;
        std::fs::write(&path, json)?;

        info!("Saved snapshot '{}' to {}", snapshot.name, path.display());
        Ok(path)
    }

    /// Load a snapshot from disk
    pub fn load(&self, name: &str) -> Result<Snapshot> {
        let path = self.snapshot_path(name);

        if !path.exists() {
            return Err(Error::SnapshotNotFound(name.to_string()));
        }

        let json = std::fs::read_to_string(&path)?;
        let snapshot: Snapshot = serde_json::from_str(&json)?;
        Ok(snapshot)
    }

    /// List all snapshots
    pub fn list(&self) -> Result<Vec<SnapshotInfo>> {
        self.ensure_dir()?;

        let mut snapshots = Vec::new();

        for entry in std::fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(snapshot) = self.load_info(&path) {
                    snapshots.push(snapshot);
                }
            }
        }

        // Sort by creation time (newest first)
        snapshots.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(snapshots)
    }

    /// Load basic snapshot info without full data
    fn load_info(&self, path: &Path) -> Result<SnapshotInfo> {
        let json = std::fs::read_to_string(path)?;
        let snapshot: Snapshot = serde_json::from_str(&json)?;

        Ok(SnapshotInfo {
            name: snapshot.name,
            description: snapshot.description,
            created_at: snapshot.created_at,
            formula_count: snapshot.formulas.len(),
            cask_count: snapshot.casks.len(),
        })
    }

    /// Delete a snapshot
    pub fn delete(&self, name: &str) -> Result<()> {
        let path = self.snapshot_path(name);

        if !path.exists() {
            return Err(Error::SnapshotNotFound(name.to_string()));
        }

        std::fs::remove_file(&path)?;
        info!("Deleted snapshot '{}'", name);
        Ok(())
    }

    /// Check if a snapshot exists
    pub fn exists(&self, name: &str) -> bool {
        self.snapshot_path(name).exists()
    }

    /// Export a snapshot to a writer
    pub fn export(&self, name: &str) -> Result<String> {
        let snapshot = self.load(name)?;
        Ok(serde_json::to_string_pretty(&snapshot)?)
    }

    /// Import a snapshot from JSON
    pub fn import(&self, json: &str) -> Result<String> {
        let snapshot: Snapshot = serde_json::from_str(json)?;
        self.save(&snapshot)?;
        Ok(snapshot.name)
    }
}

/// Basic snapshot info for listing
#[derive(Debug, Clone, Serialize)]
pub struct SnapshotInfo {
    pub name: String,
    pub description: Option<String>,
    pub created_at: String,
    pub formula_count: usize,
    pub cask_count: usize,
}

/// Generate current timestamp
fn current_timestamp() -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_snapshot_creation() {
        let mut snapshot = Snapshot::new("test", Some("Test snapshot"));

        snapshot.add_formula("jq", "1.7.1", 0, true);
        snapshot.add_formula("oniguruma", "6.9.9", 0, false);
        snapshot.add_cask("firefox", "130.0");

        assert_eq!(snapshot.formula_count(), 2);
        assert_eq!(snapshot.cask_count(), 1);
        assert_eq!(snapshot.requested_formulas(), vec!["jq"]);
    }

    #[test]
    fn test_snapshot_manager() {
        let dir = tempdir().unwrap();
        let manager = SnapshotManager::new(dir.path());

        let mut snapshot = Snapshot::new("test", Some("Test"));
        snapshot.add_formula("jq", "1.7.1", 0, true);

        // Save
        manager.save(&snapshot).unwrap();
        assert!(manager.exists("test"));

        // Load
        let loaded = manager.load("test").unwrap();
        assert_eq!(loaded.name, "test");
        assert_eq!(loaded.formula_count(), 1);

        // List
        let list = manager.list().unwrap();
        assert_eq!(list.len(), 1);

        // Delete
        manager.delete("test").unwrap();
        assert!(!manager.exists("test"));
    }
}
