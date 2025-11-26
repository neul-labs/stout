//! Index synchronization from remote

use crate::db::Database;
use crate::error::{Error, Result};
use crate::formula::Formula;
use crate::DEFAULT_INDEX_URL;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Manifest file structure
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Manifest {
    pub version: String,
    pub index_version: String,
    pub index_sha256: String,
    pub index_size: u64,
    pub formula_count: u32,
    pub created_at: String,
    pub homebrew_commit: Option<String>,
}

/// Index synchronization handler
pub struct IndexSync {
    client: Client,
    base_url: String,
    cache_dir: PathBuf,
}

impl IndexSync {
    pub fn new(base_url: Option<&str>, cache_dir: impl AsRef<Path>) -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!("brewx/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::Http)?;

        Ok(Self {
            client,
            base_url: base_url.unwrap_or(DEFAULT_INDEX_URL).to_string(),
            cache_dir: cache_dir.as_ref().to_path_buf(),
        })
    }

    /// Fetch the manifest
    pub async fn fetch_manifest(&self) -> Result<Manifest> {
        let url = format!("{}/manifest.json", self.base_url);
        debug!("Fetching manifest from {}", url);

        let response = self.client.get(&url).send().await?;
        let manifest: Manifest = response.json().await?;

        Ok(manifest)
    }

    /// Check if an update is available
    pub async fn check_update(&self, db: &Database) -> Result<Option<Manifest>> {
        let manifest = self.fetch_manifest().await?;
        let local_version = db.version()?;

        match local_version {
            Some(v) if v == manifest.version => {
                debug!("Index is up to date ({})", v);
                Ok(None)
            }
            Some(v) => {
                info!("Update available: {} -> {}", v, manifest.version);
                Ok(Some(manifest))
            }
            None => {
                info!("No local index, will download {}", manifest.version);
                Ok(Some(manifest))
            }
        }
    }

    /// Download and install the index
    pub async fn sync_index(&self, db_path: impl AsRef<Path>) -> Result<Manifest> {
        let manifest = self.fetch_manifest().await?;

        // Download compressed index
        let url = format!("{}/index.db.zst", self.base_url);
        info!("Downloading index from {}", url);

        let response = self.client.get(&url).send().await?;
        let compressed = response.bytes().await?;

        // Verify checksum
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let hash = format!("{:x}", hasher.finalize());

        if hash != manifest.index_sha256 {
            return Err(Error::ChecksumMismatch("index.db.zst".to_string()));
        }

        // Decompress
        debug!("Decompressing index ({} bytes)", compressed.len());
        let mut decoder = zstd::Decoder::new(Cursor::new(compressed))?;
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;

        // Write to temp file then rename (atomic)
        let db_path = db_path.as_ref();
        let temp_path = db_path.with_extension("db.tmp");

        std::fs::write(&temp_path, &decompressed)?;
        std::fs::rename(&temp_path, db_path)?;

        info!(
            "Index updated to {} ({} formulas)",
            manifest.version, manifest.formula_count
        );

        Ok(manifest)
    }

    /// Fetch a formula's full JSON data
    pub async fn fetch_formula(&self, name: &str) -> Result<Formula> {
        let first_char = name.chars().next().unwrap_or('_');
        let url = format!(
            "{}/formulas/{}/{}.json.zst",
            self.base_url, first_char, name
        );

        debug!("Fetching formula from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::FormulaNotFound(name.to_string()));
        }

        let compressed = response.bytes().await?;

        // Decompress
        let mut decoder = zstd::Decoder::new(Cursor::new(compressed))?;
        let mut json_bytes = Vec::new();
        decoder.read_to_end(&mut json_bytes)?;

        // Parse
        let formula: Formula = serde_json::from_slice(&json_bytes)?;

        Ok(formula)
    }

    /// Fetch and cache a formula
    pub async fn fetch_formula_cached(&self, name: &str, expected_hash: Option<&str>) -> Result<Formula> {
        let cache_path = self.cache_dir.join("formulas").join(format!("{}.json", name));

        // Check cache
        if let Some(hash) = expected_hash {
            if cache_path.exists() {
                let cached = std::fs::read_to_string(&cache_path)?;
                let mut hasher = Sha256::new();
                hasher.update(cached.as_bytes());
                let cached_hash = format!("{:x}", hasher.finalize());

                if cached_hash == hash {
                    debug!("Using cached formula for {}", name);
                    return Ok(serde_json::from_str(&cached)?);
                }
            }
        }

        // Fetch fresh
        let formula = self.fetch_formula(name).await?;

        // Cache it
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&formula)?;
        std::fs::write(&cache_path, &json)?;

        Ok(formula)
    }
}
