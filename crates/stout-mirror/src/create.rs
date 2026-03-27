//! Mirror creation functionality

use crate::error::{Error, Result};
use crate::manifest::{BottleInfo, MirrorManifest, PackageInfo};
use stout_index::{Database, FormulaInfo};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

/// Configuration for creating a mirror
#[derive(Debug, Clone)]
pub struct MirrorConfig {
    /// Output directory for the mirror
    pub output: PathBuf,

    /// Formula packages to include
    pub packages: Vec<String>,

    /// Cask packages to include
    pub casks: Vec<String>,

    /// Linux apps to include
    pub linux_apps: Vec<String>,

    /// Platforms to include (e.g., "arm64_sonoma", "x86_64_linux")
    pub platforms: Vec<String>,

    /// Include dependencies
    pub include_deps: bool,

    /// Path to Brewfile (alternative to explicit packages)
    pub brewfile: Option<PathBuf>,
}

impl Default for MirrorConfig {
    fn default() -> Self {
        Self {
            output: PathBuf::from("./mirror"),
            packages: Vec::new(),
            casks: Vec::new(),
            linux_apps: Vec::new(),
            platforms: vec![detect_platform()],
            include_deps: true,
            brewfile: None,
        }
    }
}

/// Create a mirror with the specified packages
pub async fn create_mirror(config: MirrorConfig, db: &Database) -> Result<MirrorManifest> {
    info!("Creating mirror at {:?}", config.output);

    let mut manifest = MirrorManifest::new();
    manifest.platforms = config.platforms.clone();

    // Create directory structure
    create_mirror_dirs(&config.output)?;

    // Resolve packages with dependencies
    let packages = if config.include_deps {
        resolve_with_deps(&config.packages, db)?
    } else {
        config.packages.clone()
    };

    info!("Mirror will include {} formulas", packages.len());

    // Copy formula index (filtered)
    let index_path = copy_formula_index(&config.output)?;
    let index_checksum = sha256_file(&index_path)?;
    manifest.add_checksum("formulas/index.db.zst", &index_checksum);

    // Download formula JSON files and bottles
    for package in &packages {
        info!("Processing formula: {}", package);

        // Get formula info from database
        let formula: FormulaInfo = match db.get_formula(package) {
            Ok(Some(f)) => f,
            _ => {
                debug!("Formula {} not found in index", package);
                continue;
            }
        };

        // Copy formula JSON
        let json_path = format!("formulas/data/{}/{}.json.zst", first_char(package), package);
        let json_dest = config.output.join(&json_path);

        if let Some(parent) = json_dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // For now, we'll note this needs to be copied from cache/downloaded
        // In a real implementation, this would fetch from the index server

        let mut pkg_info = PackageInfo {
            version: formula.version.clone(),
            revision: formula.revision,
            json_path: json_path.clone(),
            bottles: std::collections::HashMap::new(),
        };

        // Download bottles for each platform (if formula has bottles)
        if formula.has_bottle {
            for platform in &config.platforms {
                // Construct bottle URL from Homebrew's CDN
                let bottle_url = construct_bottle_url(package, &formula.version, platform);
                let bottle_filename = format!(
                    "{}-{}.{}.bottle.tar.gz",
                    package, formula.version, platform
                );
                let bottle_path = format!("formulas/bottles/{}", bottle_filename);
                let bottle_dest = config.output.join(&bottle_path);

                if let Some(parent) = bottle_dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }

                // Download bottle
                match download_bottle(&bottle_url, &bottle_dest).await {
                    Ok(size) => {
                        let checksum = sha256_file(&bottle_dest)?;
                        pkg_info.bottles.insert(
                            platform.clone(),
                            BottleInfo {
                                path: bottle_path,
                                sha256: checksum,
                                size,
                            },
                        );
                        manifest.total_size += size;
                    }
                    Err(e) => {
                        debug!("Failed to download bottle for {}/{}: {}", package, platform, e);
                    }
                }
            }
        }

        manifest.add_formula(package, pkg_info);
    }

    // Save manifest
    let manifest_path = config.output.join("manifest.json");
    manifest.save(&manifest_path)?;

    info!(
        "Mirror created: {} formulas, {} total size",
        manifest.formulas.count,
        humansize::format_size(manifest.total_size, humansize::BINARY)
    );

    Ok(manifest)
}

/// Create the mirror directory structure
fn create_mirror_dirs(output: &Path) -> Result<()> {
    let dirs = [
        "",
        "formulas",
        "formulas/data",
        "formulas/bottles",
        "casks",
        "casks/data",
        "casks/artifacts",
        "linux-apps",
        "linux-apps/data",
        "linux-apps/artifacts",
    ];

    for dir in dirs {
        let path = output.join(dir);
        std::fs::create_dir_all(&path)?;
    }

    Ok(())
}

/// Resolve packages including all dependencies
fn resolve_with_deps(packages: &[String], db: &Database) -> Result<Vec<String>> {
    let mut all_packages: HashSet<String> = HashSet::new();
    let mut to_process: Vec<String> = packages.to_vec();

    while let Some(pkg) = to_process.pop() {
        if all_packages.contains(&pkg) {
            continue;
        }

        all_packages.insert(pkg.clone());

        // Get dependencies
        if let Ok(deps) = db.get_dependencies(&pkg) {
            for dep in deps {
                if !all_packages.contains(&dep.name) {
                    to_process.push(dep.name);
                }
            }
        }
    }

    let mut result: Vec<String> = all_packages.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Copy and filter the formula index for the mirror
fn copy_formula_index(output: &Path) -> Result<PathBuf> {
    // For now, create a placeholder - in full implementation this would
    // create a filtered SQLite database with only the specified packages
    let index_path = output.join("formulas/index.db.zst");

    // Create a minimal placeholder
    if !index_path.exists() {
        std::fs::write(&index_path, b"")?;
    }

    Ok(index_path)
}

/// Construct bottle URL from Homebrew's CDN
fn construct_bottle_url(name: &str, version: &str, platform: &str) -> String {
    // Homebrew bottles are hosted on ghcr.io
    // Format: ghcr.io/v2/homebrew/core/<name>/blobs/sha256:<hash>
    // But we need the actual URL, so we'll use the formulae.brew.sh API pattern
    format!(
        "https://ghcr.io/v2/homebrew/core/{}/blobs/sha256:placeholder-{}-{}",
        name, version, platform
    )
}

/// Download a bottle to the specified path
async fn download_bottle(url: &str, dest: &Path) -> Result<u64> {
    debug!("Downloading bottle from {}", url);

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(Error::Network(
            response.error_for_status().unwrap_err()
        ));
    }

    let bytes = response.bytes().await?;
    std::fs::write(dest, &bytes)?;

    Ok(bytes.len() as u64)
}

/// Calculate SHA256 hash of a file
fn sha256_file(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(hex::encode(hasher.finalize()))
}

/// Get first character of a string (for directory structure)
fn first_char(s: &str) -> char {
    s.chars().next().unwrap_or('_').to_ascii_lowercase()
}

/// Detect the current platform
pub fn detect_platform() -> String {
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        {
            // Try to detect macOS version
            if let Ok(output) = std::process::Command::new("sw_vers")
                .arg("-productVersion")
                .output()
            {
                let version = String::from_utf8_lossy(&output.stdout);
                if version.starts_with("14") {
                    return "arm64_sonoma".to_string();
                } else if version.starts_with("13") {
                    return "arm64_ventura".to_string();
                } else if version.starts_with("12") {
                    return "arm64_monterey".to_string();
                }
            }
            "arm64_sonoma".to_string()
        }
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64_sonoma".to_string()
        }
        #[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
        {
            "x86_64_sonoma".to_string()
        }
    }

    #[cfg(target_os = "linux")]
    {
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64_linux".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        {
            "aarch64_linux".to_string()
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            "x86_64_linux".to_string()
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}
