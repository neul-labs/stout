//! Index synchronization from remote

use crate::cask::Cask;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::formula::{Formula, HomebrewFormula};
use crate::signature::SignatureVerifier;
use crate::DEFAULT_INDEX_URL;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Validate a package name for safe use in file paths
/// Returns an error if the name contains path traversal characters or is empty
pub fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(Error::InvalidInput(
            "package name cannot be empty".to_string(),
        ));
    }
    if name.contains("..") || name.contains('/') || name.contains('\0') {
        return Err(Error::InvalidInput(format!(
            "package name '{}' contains invalid characters for file path",
            name
        )));
    }
    Ok(())
}

/// Validate a cask token for safe use in file paths
pub fn validate_cask_token(token: &str) -> Result<()> {
    if token.is_empty() {
        return Err(Error::InvalidInput(
            "cask token cannot be empty".to_string(),
        ));
    }
    if token.contains("..") || token.contains('/') || token.contains('\0') {
        return Err(Error::InvalidInput(format!(
            "cask token '{}' contains invalid characters for file path",
            token
        )));
    }
    Ok(())
}

/// Index entry in the manifest (formulas, casks, linux_apps, etc.)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct IndexEntry {
    pub count: u32,
    pub db_sha256: String,
    pub db_size: u64,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Indexes section of the manifest
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Indexes {
    pub formulas: IndexEntry,
    #[serde(default)]
    pub casks: Option<IndexEntry>,
    #[serde(default)]
    pub linux_apps: Option<IndexEntry>,
    #[serde(default)]
    pub vulnerabilities: Option<IndexEntry>,
}

/// Manifest file structure (supports both old flat format and new nested format)
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Manifest {
    pub version: String,
    pub created_at: String,
    #[serde(default)]
    pub stout_min_version: Option<String>,

    // New nested format
    #[serde(default)]
    pub indexes: Option<Indexes>,

    // Legacy flat format fields (for backwards compatibility)
    #[serde(default)]
    pub index_version: String,
    #[serde(default)]
    pub index_sha256: Option<String>,
    #[serde(default)]
    pub index_size: Option<u64>,
    #[serde(default)]
    pub formula_count: Option<u32>,
    #[serde(default)]
    pub homebrew_commit: Option<String>,
    /// Ed25519 signature (hex-encoded) - required for verified sync
    #[serde(default)]
    pub signature: Option<String>,
    /// Unix timestamp when signed
    #[serde(default)]
    pub signed_at: Option<u64>,
    /// Cask count (optional, legacy)
    #[serde(default)]
    pub cask_count: Option<u32>,
}

impl Manifest {
    /// Get formula count from either new or legacy format
    pub fn formula_count(&self) -> u32 {
        if let Some(indexes) = &self.indexes {
            indexes.formulas.count
        } else {
            self.formula_count.unwrap_or(0)
        }
    }

    /// Get formula index SHA256 from either new or legacy format
    pub fn formula_sha256(&self) -> Option<&str> {
        if let Some(indexes) = &self.indexes {
            Some(&indexes.formulas.db_sha256)
        } else {
            self.index_sha256.as_deref()
        }
    }

    /// Get formula index size from either new or legacy format
    pub fn formula_size(&self) -> u64 {
        if let Some(indexes) = &self.indexes {
            indexes.formulas.db_size
        } else {
            self.index_size.unwrap_or(0)
        }
    }

    /// Get cask count from either new or legacy format
    pub fn cask_count(&self) -> u32 {
        if let Some(indexes) = &self.indexes {
            indexes.casks.as_ref().map(|c| c.count).unwrap_or(0)
        } else {
            self.cask_count.unwrap_or(0)
        }
    }

    /// Get cask index info if available
    pub fn cask_index(&self) -> Option<&IndexEntry> {
        self.indexes.as_ref().and_then(|i| i.casks.as_ref())
    }
}

/// Security policy for index synchronization
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Require valid signatures (default: true in release, false in debug)
    pub require_signature: bool,
    /// Maximum age of signature in seconds (default: 7 days)
    pub max_signature_age: u64,
    /// Additional trusted public keys (beyond the default)
    pub additional_keys: Vec<String>,
    /// Allow unsigned indexes (for development/testing)
    pub allow_unsigned: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            // TODO: Enable signature requirement once index server implements signing
            require_signature: false,
            max_signature_age: 7 * 24 * 60 * 60, // 7 days
            additional_keys: vec![],
            // TODO: Set to false once index server implements signing
            allow_unsigned: true,
        }
    }
}

impl SecurityPolicy {
    /// Create a strict policy (always require signatures)
    pub fn strict() -> Self {
        Self {
            require_signature: true,
            max_signature_age: 24 * 60 * 60, // 1 day
            additional_keys: vec![],
            allow_unsigned: false,
        }
    }

    /// Create a permissive policy (for development)
    pub fn permissive() -> Self {
        Self {
            require_signature: false,
            max_signature_age: u64::MAX,
            additional_keys: vec![],
            allow_unsigned: true,
        }
    }
}

/// Index synchronization handler
pub struct IndexSync {
    client: Client,
    base_url: String,
    cache_dir: PathBuf,
    security_policy: SecurityPolicy,
    verifier: SignatureVerifier,
}

impl IndexSync {
    /// Create a new IndexSync with default security policy
    pub fn new(base_url: Option<&str>, cache_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_security_policy(base_url, cache_dir, SecurityPolicy::default())
    }

    /// Create a new IndexSync with a specific security policy
    pub fn with_security_policy(
        base_url: Option<&str>,
        cache_dir: impl AsRef<Path>,
        security_policy: SecurityPolicy,
    ) -> Result<Self> {
        let base_url = base_url.unwrap_or(DEFAULT_INDEX_URL);

        // Security: Validate URL
        Self::validate_base_url(base_url, &security_policy)?;

        let client = Client::builder()
            .user_agent(concat!("stout/", env!("CARGO_PKG_VERSION")))
            // Enforce TLS 1.2+ (reqwest default, but explicit for security)
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            .build()
            .map_err(Error::Http)?;

        // Build verifier with default key + any additional trusted keys
        let mut verifier = SignatureVerifier::new()?;
        for key in &security_policy.additional_keys {
            verifier.add_public_key(key)?;
        }

        Ok(Self {
            client,
            base_url: base_url.to_string(),
            cache_dir: cache_dir.as_ref().to_path_buf(),
            security_policy,
            verifier,
        })
    }

    /// Validate the base URL for security
    ///
    /// Note: Domain validation is intentionally permissive because signature
    /// verification is the primary security mechanism. Mirrors can be hosted
    /// anywhere as long as they serve properly signed manifests.
    fn validate_base_url(url: &str, policy: &SecurityPolicy) -> Result<()> {
        // In permissive mode, allow any URL (for development)
        if policy.allow_unsigned {
            return Ok(());
        }

        // Allow file:// URLs for local mirrors
        if url.starts_with("file://") {
            debug!("Using local file mirror: {}", url);
            return Ok(());
        }

        // Require HTTPS for remote URLs (not file://)
        if !url.starts_with("https://") {
            warn!("Index URL does not use HTTPS: {}", url);
            // Only error if signatures are required - the signature check is
            // the real security, HTTPS is defense in depth
            if policy.require_signature {
                return Err(Error::InvalidIndex(
                    "Remote index URL must use HTTPS. Use file:// for local mirrors.".to_string(),
                ));
            }
        }

        // No domain restrictions - signature verification protects against
        // untrusted sources. This allows:
        // - Enterprise mirrors on internal domains
        // - CDN-hosted mirrors
        // - Local development servers
        // The Ed25519 signature is what actually validates authenticity.

        Ok(())
    }

    /// Create with permissive security (for development/testing)
    pub fn permissive(base_url: Option<&str>, cache_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_security_policy(base_url, cache_dir, SecurityPolicy::permissive())
    }

    /// Create with strict security (production recommended)
    pub fn strict(base_url: Option<&str>, cache_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_security_policy(base_url, cache_dir, SecurityPolicy::strict())
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

    /// Verify manifest signature according to security policy
    fn verify_manifest_signature(&self, manifest: &Manifest) -> Result<()> {
        // Check if signature is present
        let signature = match &manifest.signature {
            Some(sig) => sig,
            None => {
                if self.security_policy.allow_unsigned {
                    warn!("Manifest is unsigned, but policy allows unsigned indexes");
                    return Ok(());
                }
                return Err(Error::SignatureMissing);
            }
        };

        let signed_at = manifest.signed_at.unwrap_or(0);

        // Check signature age
        if self.security_policy.require_signature {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            let age = now.saturating_sub(signed_at);
            if age > self.security_policy.max_signature_age {
                return Err(Error::SignatureExpired(
                    age,
                    self.security_policy.max_signature_age,
                ));
            }
        }

        // Build signed data string (must match sign_index.py format)
        // Use accessor methods to support both old and new manifest formats
        let index_sha256 = manifest.formula_sha256().unwrap_or("");
        let signed_data = format!(
            "stout-index:v1:{}:{}:{}:{}:{}",
            index_sha256,
            signed_at,
            manifest.index_version,
            manifest.formula_count(),
            manifest.cask_count()
        );

        // Parse and verify signature
        let signature_bytes = hex::decode(signature)
            .map_err(|e| Error::SignatureInvalid(format!("Invalid signature hex: {}", e)))?;

        let sig = ed25519_dalek::Signature::from_slice(&signature_bytes)
            .map_err(|e| Error::SignatureInvalid(format!("Invalid signature format: {}", e)))?;

        // Try verification with the verifier (which has all trusted keys)
        use ed25519_dalek::Verifier;
        let verified = self
            .verifier
            .public_keys()
            .iter()
            .any(|key| key.verify(signed_data.as_bytes(), &sig).is_ok());

        if !verified {
            return Err(Error::SignatureInvalid(
                "Signature verification failed".to_string(),
            ));
        }

        info!("Manifest signature verified successfully");
        Ok(())
    }

    /// Download and install the index
    pub async fn sync_index(&self, db_path: impl AsRef<Path>) -> Result<Manifest> {
        let manifest = self.fetch_manifest().await?;

        // Verify signature according to security policy
        self.verify_manifest_signature(&manifest)?;

        // Download compressed formula index
        let url = format!("{}/formulas/index.db.zst", self.base_url);
        info!("Downloading formula index from {}", url);

        let response = self.client.get(&url).send().await?;
        let compressed = response.bytes().await?;

        // Verify checksum
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let hash = hex::encode(hasher.finalize());

        let expected_hash = manifest
            .formula_sha256()
            .ok_or_else(|| Error::InvalidIndex("No formula index hash in manifest".to_string()))?;

        if hash != expected_hash {
            return Err(Error::ChecksumMismatch("formulas.db.zst".to_string()));
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
            manifest.version,
            manifest.formula_count()
        );

        Ok(manifest)
    }

    /// Fetch a formula's full JSON data
    ///
    /// Tries the stout-index first, then falls back to Homebrew's official API
    /// if the formula is not found in the index.
    pub async fn fetch_formula(&self, name: &str) -> Result<Formula> {
        // Try stout-index first
        match self.fetch_formula_from_index(name).await {
            Ok(formula) => return Ok(formula),
            Err(Error::FormulaNotFound(_)) => {
                // Fall back to Homebrew API
                debug!(
                    "Formula {} not found in stout-index, trying Homebrew API",
                    name
                );
                self.fetch_formula_from_homebrew(name).await
            }
            Err(e) => return Err(e),
        }
    }

    /// Fetch a formula from the stout-index mirror
    async fn fetch_formula_from_index(&self, name: &str) -> Result<Formula> {
        let first_char = name.chars().next().unwrap_or('_');
        let url = format!(
            "{}/formulas/data/{}/{}.json.zst",
            self.base_url, first_char, name
        );

        debug!("Fetching formula from stout-index: {}", url);

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

    /// Fetch a formula from Homebrew's official API (fallback)
    async fn fetch_formula_from_homebrew(&self, name: &str) -> Result<Formula> {
        let url = format!("https://formulae.brew.sh/api/formula/{}.json", name);

        debug!("Fetching formula from Homebrew API: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::FormulaNotFound(name.to_string()));
        }

        let json_bytes = response.bytes().await?;

        // Parse Homebrew API format and convert to stout format
        let homebrew_formula: HomebrewFormula =
            serde_json::from_slice(&json_bytes).map_err(|e| {
                warn!("Failed to parse Homebrew API response for {}: {}", name, e);
                Error::Json(e)
            })?;

        Ok(Formula::from(homebrew_formula))
    }

    /// Fetch and cache a formula
    pub async fn fetch_formula_cached(
        &self,
        name: &str,
        expected_hash: Option<&str>,
    ) -> Result<Formula> {
        // Validate package name to prevent path traversal
        validate_package_name(name)?;

        let cache_path = self
            .cache_dir
            .join("formulas")
            .join(format!("{}.json", name));

        // Check cache
        if let Some(hash) = expected_hash {
            if cache_path.exists() {
                let cached = std::fs::read_to_string(&cache_path)?;
                let mut hasher = Sha256::new();
                hasher.update(cached.as_bytes());
                let cached_hash = hex::encode(hasher.finalize());

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

    /// Fetch a cask's full JSON data
    pub async fn fetch_cask(&self, token: &str) -> Result<Cask> {
        let first_char = token.chars().next().unwrap_or('_');
        let url = format!(
            "{}/casks/data/{}/{}.json.zst",
            self.base_url, first_char, token
        );

        debug!("Fetching cask from {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(Error::CaskNotFound(token.to_string()));
        }

        let compressed = response.bytes().await?;

        // Decompress
        let mut decoder = zstd::Decoder::new(Cursor::new(compressed))?;
        let mut json_bytes = Vec::new();
        decoder.read_to_end(&mut json_bytes)?;

        // Parse
        let cask: Cask = serde_json::from_slice(&json_bytes)?;

        Ok(cask)
    }

    /// Fetch and cache a cask
    pub async fn fetch_cask_cached(
        &self,
        token: &str,
        expected_hash: Option<&str>,
    ) -> Result<Cask> {
        // Validate cask token to prevent path traversal
        validate_cask_token(token)?;

        let cache_path = self.cache_dir.join("casks").join(format!("{}.json", token));

        // Check cache
        if let Some(hash) = expected_hash {
            if cache_path.exists() {
                let cached = std::fs::read_to_string(&cache_path)?;
                let mut hasher = Sha256::new();
                hasher.update(cached.as_bytes());
                let cached_hash = hex::encode(hasher.finalize());

                if cached_hash == hash {
                    debug!("Using cached cask for {}", token);
                    return Ok(serde_json::from_str(&cached)?);
                }
            }
        }

        // Fetch fresh
        let cask = self.fetch_cask(token).await?;

        // Cache it
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&cask)?;
        std::fs::write(&cache_path, &json)?;

        Ok(cask)
    }
}
