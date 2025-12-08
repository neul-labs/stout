//! Mirror manifest types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Mirror manifest containing package metadata and checksums
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorManifest {
    /// Version of the manifest format
    pub version: String,

    /// When the mirror was created
    pub created_at: String,

    /// stout version used to create the mirror
    pub stout_version: String,

    /// Platforms included in this mirror
    pub platforms: Vec<String>,

    /// Formula packages
    #[serde(default)]
    pub formulas: FormulaManifest,

    /// Cask packages (macOS apps)
    #[serde(default)]
    pub casks: CaskManifest,

    /// Linux apps
    #[serde(default)]
    pub linux_apps: LinuxAppManifest,

    /// Index file checksums
    #[serde(default)]
    pub checksums: HashMap<String, String>,

    /// Total size of the mirror in bytes
    #[serde(default)]
    pub total_size: u64,

    /// Upstream index signature (copied from source)
    /// This allows verification that the mirror contains authentic data
    #[serde(default)]
    pub upstream_signature: Option<UpstreamSignature>,

    /// Mirror signature (optional, for enterprise deployments)
    /// Enterprises can sign their mirrors with their own keys
    #[serde(default)]
    pub mirror_signature: Option<String>,

    /// Unix timestamp when mirror was signed
    #[serde(default)]
    pub signed_at: Option<u64>,
}

/// Upstream signature information copied from the original index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpstreamSignature {
    /// Original index SHA256
    pub index_sha256: String,
    /// Original signature
    pub signature: String,
    /// Original signed_at timestamp
    pub signed_at: u64,
    /// Original index version
    pub index_version: String,
    /// Formula count at time of signing
    pub formula_count: u32,
    /// Cask count at time of signing
    pub cask_count: u32,
}

impl MirrorManifest {
    /// Create a new empty manifest
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            created_at: chrono_lite_now(),
            stout_version: env!("CARGO_PKG_VERSION").to_string(),
            platforms: Vec::new(),
            formulas: FormulaManifest::default(),
            casks: CaskManifest::default(),
            linux_apps: LinuxAppManifest::default(),
            checksums: HashMap::new(),
            total_size: 0,
            upstream_signature: None,
            mirror_signature: None,
            signed_at: None,
        }
    }

    /// Set the upstream signature from the original index manifest
    pub fn set_upstream_signature(&mut self, sig: UpstreamSignature) {
        self.upstream_signature = Some(sig);
    }

    /// Check if the mirror has a valid upstream signature
    pub fn has_upstream_signature(&self) -> bool {
        self.upstream_signature.is_some()
    }

    /// Check if the mirror has been signed by the mirror operator
    pub fn has_mirror_signature(&self) -> bool {
        self.mirror_signature.is_some()
    }

    /// Load manifest from file
    pub fn load(path: &Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let manifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    /// Save manifest to file
    pub fn save(&self, path: &Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add a formula to the manifest
    pub fn add_formula(&mut self, name: &str, info: PackageInfo) {
        self.formulas.count += 1;
        self.formulas.packages.insert(name.to_string(), info);
    }

    /// Add a cask to the manifest
    pub fn add_cask(&mut self, token: &str, info: CaskInfo) {
        self.casks.count += 1;
        self.casks.packages.insert(token.to_string(), info);
    }

    /// Get a formula from the manifest
    pub fn get_formula(&self, name: &str) -> Option<&PackageInfo> {
        self.formulas.packages.get(name)
    }

    /// Get a cask from the manifest
    pub fn get_cask(&self, token: &str) -> Option<&CaskInfo> {
        self.casks.packages.get(token)
    }

    /// Add a checksum for a file
    pub fn add_checksum(&mut self, path: &str, checksum: &str) {
        self.checksums.insert(path.to_string(), checksum.to_string());
    }
}

impl Default for MirrorManifest {
    fn default() -> Self {
        Self::new()
    }
}

/// Formula manifest section
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FormulaManifest {
    pub count: usize,
    #[serde(default)]
    pub packages: HashMap<String, PackageInfo>,
}

/// Cask manifest section
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CaskManifest {
    pub count: usize,
    #[serde(default)]
    pub packages: HashMap<String, CaskInfo>,
}

/// Linux app manifest section
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LinuxAppManifest {
    pub count: usize,
    #[serde(default)]
    pub packages: HashMap<String, LinuxAppInfo>,
}

/// Formula package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub version: String,
    #[serde(default)]
    pub revision: u32,
    pub json_path: String,
    #[serde(default)]
    pub bottles: HashMap<String, BottleInfo>,
}

/// Bottle (binary package) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottleInfo {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// Cask package information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaskInfo {
    pub version: String,
    pub json_path: String,
    #[serde(default)]
    pub artifact: Option<ArtifactInfo>,
}

/// Artifact (DMG/PKG/ZIP) information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// Linux app information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinuxAppInfo {
    pub json_path: String,
    #[serde(default)]
    pub appimage: Option<ArtifactInfo>,
    #[serde(default)]
    pub flatpak_id: Option<String>,
}

/// Simple timestamp without pulling in chrono
fn chrono_lite_now() -> String {
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
