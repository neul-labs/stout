//! Package installation history tracking

use crate::error::Result;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Action that was performed on a package
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryAction {
    Install,
    Upgrade,
    Downgrade,
    Reinstall,
    Uninstall,
}

impl HistoryAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Upgrade => "upgrade",
            Self::Downgrade => "downgrade",
            Self::Reinstall => "reinstall",
            Self::Uninstall => "uninstall",
        }
    }
}

/// A single history entry for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub version: String,
    pub revision: u32,
    pub action: HistoryAction,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_revision: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bottle_path: Option<String>,
}

/// Package history storage
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PackageHistory {
    #[serde(default)]
    pub packages: HashMap<String, Vec<HistoryEntry>>,
}

impl PackageHistory {
    /// Load history from file
    pub fn load(paths: &Paths) -> Result<Self> {
        let file_path = paths.history_file();

        if file_path.exists() {
            let contents = std::fs::read_to_string(&file_path)?;
            let history: PackageHistory = serde_json::from_str(&contents)?;
            Ok(history)
        } else {
            Ok(Self::default())
        }
    }

    /// Save history to file
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let file_path = paths.history_file();

        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        std::fs::write(&file_path, contents)?;
        Ok(())
    }

    /// Record an install action
    pub fn record_install(&mut self, name: &str, version: &str, revision: u32) {
        self.record(name, version, revision, HistoryAction::Install, None, None);
    }

    /// Record an upgrade action
    pub fn record_upgrade(
        &mut self,
        name: &str,
        version: &str,
        revision: u32,
        from_version: &str,
        from_revision: u32,
    ) {
        self.record(
            name,
            version,
            revision,
            HistoryAction::Upgrade,
            Some(from_version.to_string()),
            Some(from_revision),
        );
    }

    /// Record a downgrade action
    pub fn record_downgrade(
        &mut self,
        name: &str,
        version: &str,
        revision: u32,
        from_version: &str,
        from_revision: u32,
    ) {
        self.record(
            name,
            version,
            revision,
            HistoryAction::Downgrade,
            Some(from_version.to_string()),
            Some(from_revision),
        );
    }

    /// Record a reinstall action
    pub fn record_reinstall(&mut self, name: &str, version: &str, revision: u32) {
        self.record(
            name,
            version,
            revision,
            HistoryAction::Reinstall,
            None,
            None,
        );
    }

    /// Record an uninstall action
    pub fn record_uninstall(&mut self, name: &str, version: &str, revision: u32) {
        self.record(
            name,
            version,
            revision,
            HistoryAction::Uninstall,
            None,
            None,
        );
    }

    /// Record a history entry
    fn record(
        &mut self,
        name: &str,
        version: &str,
        revision: u32,
        action: HistoryAction,
        from_version: Option<String>,
        from_revision: Option<u32>,
    ) {
        let entry = HistoryEntry {
            version: version.to_string(),
            revision,
            action,
            timestamp: chrono_lite_now(),
            from_version,
            from_revision,
            bottle_path: None,
        };

        self.packages
            .entry(name.to_string())
            .or_default()
            .push(entry);
    }

    /// Get history for a specific package
    pub fn get(&self, name: &str) -> Option<&Vec<HistoryEntry>> {
        self.packages.get(name)
    }

    /// Get the most recent entry for a package
    pub fn get_latest(&self, name: &str) -> Option<&HistoryEntry> {
        self.packages.get(name).and_then(|entries| entries.last())
    }

    /// Get the previous version for a package (before current)
    pub fn get_previous(&self, name: &str) -> Option<&HistoryEntry> {
        self.packages.get(name).and_then(|entries| {
            if entries.len() >= 2 {
                // Find the most recent non-uninstall entry before the last one
                entries
                    .iter()
                    .rev()
                    .skip(1)
                    .find(|e| e.action != HistoryAction::Uninstall)
            } else {
                None
            }
        })
    }

    /// Get all versions that were installed for a package
    pub fn get_installed_versions(&self, name: &str) -> Vec<(String, u32)> {
        self.packages
            .get(name)
            .map(|entries| {
                let mut versions: Vec<(String, u32)> = entries
                    .iter()
                    .filter(|e| e.action != HistoryAction::Uninstall)
                    .map(|e| (e.version.clone(), e.revision))
                    .collect();
                versions.dedup();
                versions
            })
            .unwrap_or_default()
    }

    /// Check if a package has any history
    pub fn has_history(&self, name: &str) -> bool {
        self.packages
            .get(name)
            .map(|e| !e.is_empty())
            .unwrap_or(false)
    }

    /// Prune history to keep only the last N entries per package
    pub fn prune(&mut self, keep: usize) {
        for entries in self.packages.values_mut() {
            if entries.len() > keep {
                let start = entries.len() - keep;
                *entries = entries.drain(start..).collect();
            }
        }
    }

    /// Remove all history for a package
    pub fn remove(&mut self, name: &str) {
        self.packages.remove(name);
    }
}

/// Simple timestamp without pulling in chrono
fn chrono_lite_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    let secs = duration.as_secs();

    // Simple ISO 8601 format
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Approximate year calculation (doesn't account for leap years perfectly)
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

    #[test]
    fn test_record_history() {
        let mut history = PackageHistory::default();

        history.record_install("jq", "1.7", 0);
        history.record_upgrade("jq", "1.7.1", 0, "1.7", 0);

        let entries = history.get("jq").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].action, HistoryAction::Install);
        assert_eq!(entries[1].action, HistoryAction::Upgrade);
        assert_eq!(entries[1].from_version, Some("1.7".to_string()));
    }

    #[test]
    fn test_get_previous() {
        let mut history = PackageHistory::default();

        history.record_install("jq", "1.6", 0);
        history.record_upgrade("jq", "1.7", 0, "1.6", 0);
        history.record_upgrade("jq", "1.7.1", 0, "1.7", 0);

        let prev = history.get_previous("jq").unwrap();
        assert_eq!(prev.version, "1.7");
    }

    #[test]
    fn test_prune() {
        let mut history = PackageHistory::default();

        history.record_install("jq", "1.5", 0);
        history.record_upgrade("jq", "1.6", 0, "1.5", 0);
        history.record_upgrade("jq", "1.7", 0, "1.6", 0);
        history.record_upgrade("jq", "1.7.1", 0, "1.7", 0);

        history.prune(2);

        let entries = history.get("jq").unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].version, "1.7");
        assert_eq!(entries[1].version, "1.7.1");
    }
}
