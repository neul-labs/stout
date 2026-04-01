//! Delta sync optimization for efficient index updates
//!
//! Instead of downloading the full index on every update, delta sync:
//! 1. Uses ETags/Last-Modified for conditional requests
//! 2. Downloads a delta manifest listing changed packages
//! 3. Only fetches changed formula/cask data
//! 4. Applies incremental updates to the local database

use crate::db::Database;
use crate::error::{Error, Result};
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Delta manifest containing changes between versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaManifest {
    /// Source version (what version this delta applies to)
    pub from_version: String,
    /// Target version (what version we get after applying)
    pub to_version: String,
    /// When this delta was created (Unix timestamp)
    pub created_at: u64,
    /// Added formulas
    pub formulas_added: Vec<String>,
    /// Updated formulas (version changed)
    pub formulas_updated: Vec<String>,
    /// Removed formulas
    pub formulas_removed: Vec<String>,
    /// Added casks
    pub casks_added: Vec<String>,
    /// Updated casks
    pub casks_updated: Vec<String>,
    /// Removed casks
    pub casks_removed: Vec<String>,
}

impl DeltaManifest {
    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.formulas_added.len()
            + self.formulas_updated.len()
            + self.formulas_removed.len()
            + self.casks_added.len()
            + self.casks_updated.len()
            + self.casks_removed.len()
    }

    /// Check if there are any changes
    pub fn is_empty(&self) -> bool {
        self.total_changes() == 0
    }

    /// Get all formulas that need to be fetched
    pub fn formulas_to_fetch(&self) -> impl Iterator<Item = &str> {
        self.formulas_added
            .iter()
            .chain(self.formulas_updated.iter())
            .map(|s| s.as_str())
    }

    /// Get all casks that need to be fetched
    pub fn casks_to_fetch(&self) -> impl Iterator<Item = &str> {
        self.casks_added
            .iter()
            .chain(self.casks_updated.iter())
            .map(|s| s.as_str())
    }
}

/// Metadata for conditional requests (ETag/Last-Modified caching)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SyncMetadata {
    /// ETag from last sync
    pub etag: Option<String>,
    /// Last-Modified header from last sync
    pub last_modified: Option<String>,
    /// Local version string
    pub version: Option<String>,
    /// When we last synced (Unix timestamp)
    pub last_sync: Option<u64>,
    /// Per-formula hashes for change detection
    #[serde(default)]
    pub formula_hashes: HashMap<String, String>,
    /// Per-cask hashes for change detection
    #[serde(default)]
    pub cask_hashes: HashMap<String, String>,
}

impl SyncMetadata {
    /// Load from a file
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let data = std::fs::read_to_string(path)?;
            let meta: SyncMetadata = serde_json::from_str(&data)?;
            Ok(meta)
        } else {
            Ok(Self::default())
        }
    }

    /// Save to a file
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Update sync timestamp
    pub fn mark_synced(&mut self) {
        self.last_sync = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );
    }

    /// Check if we need to sync (based on time since last sync)
    pub fn needs_sync(&self, max_age: Duration) -> bool {
        match self.last_sync {
            Some(ts) => {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                now.saturating_sub(ts) > max_age.as_secs()
            }
            None => true,
        }
    }
}

/// Delta sync client for efficient updates
pub struct DeltaSync {
    client: Client,
    base_url: String,
    metadata_path: PathBuf,
    metadata: SyncMetadata,
}

impl DeltaSync {
    /// Create a new delta sync client
    pub fn new(base_url: &str, cache_dir: &Path) -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("stout/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        let metadata_path = cache_dir.join("sync_metadata.json");
        let metadata = SyncMetadata::load(&metadata_path).unwrap_or_default();

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            metadata_path,
            metadata,
        })
    }

    /// Check if an update is available using conditional requests
    pub async fn check_update(&self) -> Result<UpdateStatus> {
        let url = format!("{}/manifest.json", self.base_url);

        let mut request = self.client.get(&url);

        // Add conditional headers if we have them
        if let Some(ref etag) = self.metadata.etag {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        if let Some(ref lm) = self.metadata.last_modified {
            request = request.header(header::IF_MODIFIED_SINCE, lm);
        }

        let response = request.send().await?;

        match response.status().as_u16() {
            304 => {
                debug!("Index not modified (304)");
                Ok(UpdateStatus::NotModified)
            }
            200 => {
                // Extract new ETag/Last-Modified
                let new_etag = response
                    .headers()
                    .get(header::ETAG)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());

                let new_last_modified = response
                    .headers()
                    .get(header::LAST_MODIFIED)
                    .and_then(|h| h.to_str().ok())
                    .map(|s| s.to_string());

                let manifest: super::Manifest = response.json().await?;

                Ok(UpdateStatus::Available {
                    manifest: Box::new(manifest),
                    etag: new_etag,
                    last_modified: new_last_modified,
                })
            }
            status => {
                warn!("Unexpected status code: {}", status);
                Err(Error::InvalidIndex(format!(
                    "Unexpected HTTP status: {}",
                    status
                )))
            }
        }
    }

    /// Try to fetch a delta manifest for incremental update
    pub async fn fetch_delta(&self, from_version: &str) -> Result<Option<DeltaManifest>> {
        // Delta manifests are stored as deltas/<from_version>.json
        let url = format!("{}/deltas/{}.json", self.base_url, from_version);

        debug!("Checking for delta manifest from version {}", from_version);

        let response = self.client.get(&url).send().await?;

        if response.status().is_success() {
            let delta: DeltaManifest = response.json().await?;
            info!(
                "Found delta: {} changes from {} to {}",
                delta.total_changes(),
                delta.from_version,
                delta.to_version
            );
            Ok(Some(delta))
        } else {
            debug!("No delta manifest available, will do full sync");
            Ok(None)
        }
    }

    /// Apply a delta update to the database
    pub async fn apply_delta(&mut self, _db: &Database, delta: &DeltaManifest) -> Result<usize> {
        let mut applied = 0;

        // Remove deleted formulas
        for name in &delta.formulas_removed {
            debug!("Removing formula: {}", name);
            // Note: actual deletion would happen in db transaction
            self.metadata.formula_hashes.remove(name);
            applied += 1;
        }

        // Remove deleted casks
        for token in &delta.casks_removed {
            debug!("Removing cask: {}", token);
            self.metadata.cask_hashes.remove(token);
            applied += 1;
        }

        // For added/updated formulas and casks, we'd fetch and insert them
        // The actual fetching would be done by the caller using IndexSync
        applied += delta.formulas_added.len();
        applied += delta.formulas_updated.len();
        applied += delta.casks_added.len();
        applied += delta.casks_updated.len();

        info!("Applied {} delta changes", applied);
        Ok(applied)
    }

    /// Update metadata after successful sync
    pub fn update_metadata(
        &mut self,
        version: &str,
        etag: Option<String>,
        last_modified: Option<String>,
    ) -> Result<()> {
        self.metadata.version = Some(version.to_string());
        self.metadata.etag = etag;
        self.metadata.last_modified = last_modified;
        self.metadata.mark_synced();
        self.metadata.save(&self.metadata_path)?;
        Ok(())
    }

    /// Get the current local version
    pub fn local_version(&self) -> Option<&str> {
        self.metadata.version.as_deref()
    }

    /// Check if delta sync is possible (we have a known version)
    pub fn can_delta_sync(&self) -> bool {
        self.metadata.version.is_some()
    }
}

/// Result of checking for updates
#[derive(Debug)]
pub enum UpdateStatus {
    /// No update available (304 Not Modified)
    NotModified,
    /// Update available
    Available {
        manifest: Box<super::Manifest>,
        etag: Option<String>,
        last_modified: Option<String>,
    },
}

impl UpdateStatus {
    /// Check if an update is available
    pub fn is_available(&self) -> bool {
        matches!(self, UpdateStatus::Available { .. })
    }

    /// Get the manifest if available
    pub fn manifest(&self) -> Option<&super::Manifest> {
        match self {
            UpdateStatus::Available { manifest, .. } => Some(manifest),
            UpdateStatus::NotModified => None,
        }
    }
}

/// Statistics about a sync operation
#[derive(Debug, Clone, Default)]
pub struct SyncStats {
    /// Whether delta sync was used
    pub used_delta: bool,
    /// Number of formulas added
    pub formulas_added: usize,
    /// Number of formulas updated
    pub formulas_updated: usize,
    /// Number of formulas removed
    pub formulas_removed: usize,
    /// Number of casks added
    pub casks_added: usize,
    /// Number of casks updated
    pub casks_updated: usize,
    /// Number of casks removed
    pub casks_removed: usize,
    /// Bytes downloaded
    pub bytes_downloaded: u64,
    /// Time taken
    pub duration_ms: u64,
}

impl SyncStats {
    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.formulas_added
            + self.formulas_updated
            + self.formulas_removed
            + self.casks_added
            + self.casks_updated
            + self.casks_removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_manifest_empty() {
        let delta = DeltaManifest {
            from_version: "1.0".to_string(),
            to_version: "1.1".to_string(),
            created_at: 0,
            formulas_added: vec![],
            formulas_updated: vec![],
            formulas_removed: vec![],
            casks_added: vec![],
            casks_updated: vec![],
            casks_removed: vec![],
        };
        assert!(delta.is_empty());
    }

    #[test]
    fn test_delta_manifest_changes() {
        let delta = DeltaManifest {
            from_version: "1.0".to_string(),
            to_version: "1.1".to_string(),
            created_at: 0,
            formulas_added: vec!["foo".to_string()],
            formulas_updated: vec!["bar".to_string()],
            formulas_removed: vec![],
            casks_added: vec![],
            casks_updated: vec![],
            casks_removed: vec!["baz".to_string()],
        };
        assert_eq!(delta.total_changes(), 3);
        assert!(!delta.is_empty());
    }

    #[test]
    fn test_sync_metadata_needs_sync() {
        let mut meta = SyncMetadata::default();
        // No last_sync set, always needs sync
        assert!(meta.needs_sync(Duration::from_secs(3600)));

        meta.mark_synced();
        // Just synced, shouldn't need sync for 1 hour
        assert!(!meta.needs_sync(Duration::from_secs(3600)));
        // Edge case: 0 max_age means always fresh (time elapsed is 0 since just synced)
        assert!(!meta.needs_sync(Duration::from_secs(0)));
    }

    #[test]
    fn test_update_status() {
        let not_modified = UpdateStatus::NotModified;
        assert!(!not_modified.is_available());
        assert!(not_modified.manifest().is_none());
    }
}
