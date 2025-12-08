//! Tap management for custom formula repositories

use crate::error::Result;
use crate::paths::Paths;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tap (custom formula repository)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tap {
    /// Tap name (e.g., "user/repo")
    pub name: String,
    /// Base URL for the tap's index
    pub url: String,
    /// Whether this tap is pinned (prevents updates)
    #[serde(default)]
    pub pinned: bool,
}

/// Manages installed taps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapManager {
    #[serde(default)]
    taps: HashMap<String, Tap>,
}

impl Default for TapManager {
    fn default() -> Self {
        let mut taps = HashMap::new();

        // Default tap is the stout-index
        taps.insert(
            "neul-labs/stout-index".to_string(),
            Tap {
                name: "neul-labs/stout-index".to_string(),
                url: "https://raw.githubusercontent.com/neul-labs/stout-index/main".to_string(),
                pinned: false,
            },
        );

        Self { taps }
    }
}

impl TapManager {
    /// Load taps from file
    pub fn load(paths: &Paths) -> Result<Self> {
        let taps_file = paths.stout_dir.join("taps.toml");

        if taps_file.exists() {
            let contents = std::fs::read_to_string(&taps_file)?;
            let manager: TapManager = toml::from_str(&contents)?;
            Ok(manager)
        } else {
            Ok(Self::default())
        }
    }

    /// Save taps to file
    pub fn save(&self, paths: &Paths) -> Result<()> {
        let taps_file = paths.stout_dir.join("taps.toml");

        if let Some(parent) = taps_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let contents = toml::to_string_pretty(self)?;
        std::fs::write(&taps_file, contents)?;
        Ok(())
    }

    /// Add a tap
    pub fn add(&mut self, tap: Tap) {
        self.taps.insert(tap.name.clone(), tap);
    }

    /// Remove a tap
    pub fn remove(&mut self, name: &str) {
        self.taps.remove(name);
    }

    /// Get a tap by name
    pub fn get(&self, name: &str) -> Option<&Tap> {
        self.taps.get(name)
    }

    /// List all taps
    pub fn list(&self) -> Vec<&Tap> {
        let mut taps: Vec<_> = self.taps.values().collect();
        taps.sort_by(|a, b| a.name.cmp(&b.name));
        taps
    }

    /// Get all tap URLs
    pub fn urls(&self) -> Vec<(&str, &str)> {
        self.taps
            .iter()
            .map(|(name, tap)| (name.as_str(), tap.url.as_str()))
            .collect()
    }

    /// Check if a tap exists
    pub fn contains(&self, name: &str) -> bool {
        self.taps.contains_key(name)
    }

    /// Pin a tap (prevent updates)
    pub fn pin(&mut self, name: &str) -> bool {
        if let Some(tap) = self.taps.get_mut(name) {
            tap.pinned = true;
            true
        } else {
            false
        }
    }

    /// Unpin a tap
    pub fn unpin(&mut self, name: &str) -> bool {
        if let Some(tap) = self.taps.get_mut(name) {
            tap.pinned = false;
            true
        } else {
            false
        }
    }
}
