//! Tests for stout-state

use crate::config::{CacheConfig, Config, IndexConfig, InstallConfig};
use crate::error::Error;
use crate::installed::{InstalledPackage, InstalledPackages};
use crate::paths::Paths;
use tempfile::tempdir;

// ============================================================================
// Config tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = Config::default();

    // Check default values
    assert!(config.index.base_url.contains("neul-labs/stout-index"));
    assert!(config.index.auto_update);
    assert_eq!(config.index.update_interval, 1800);

    assert_eq!(config.install.parallel_downloads, 4);
    assert_eq!(config.install.prefix, "/opt/homebrew");
    assert_eq!(config.install.cellar, "/opt/homebrew/Cellar");

    assert_eq!(config.cache.max_size, "2GB");
    assert_eq!(config.cache.formula_ttl, 86400);
    assert_eq!(config.cache.download_ttl, 604800);
}

#[test]
fn test_index_config_default() {
    let config = IndexConfig::default();
    assert!(config.auto_update);
    assert_eq!(config.update_interval, 1800);
}

#[test]
fn test_install_config_default() {
    let config = InstallConfig::default();
    assert_eq!(config.parallel_downloads, 4);
}

#[test]
fn test_cache_config_default() {
    let config = CacheConfig::default();
    assert_eq!(config.max_size, "2GB");
}

#[test]
fn test_config_save_and_load() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let mut config = Config::default();
    config.install.parallel_downloads = 8;
    config.index.auto_update = false;

    config.save(&paths).unwrap();

    let loaded = Config::load(&paths).unwrap();
    assert_eq!(loaded.install.parallel_downloads, 8);
    assert!(!loaded.index.auto_update);
}

#[test]
fn test_config_load_nonexistent_returns_default() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let config = Config::load(&paths).unwrap();
    assert_eq!(config.install.parallel_downloads, 4);
}

#[test]
fn test_config_serialization() {
    let config = Config::default();
    let toml_str = toml::to_string_pretty(&config).unwrap();

    assert!(toml_str.contains("[index]"));
    assert!(toml_str.contains("[install]"));
    assert!(toml_str.contains("[cache]"));
    assert!(toml_str.contains("auto_update"));
    assert!(toml_str.contains("parallel_downloads"));
}

#[test]
fn test_config_deserialization() {
    let toml_str = r#"
[index]
base_url = "https://custom.example.com"
auto_update = false
update_interval = 3600

[install]
cellar = "/custom/Cellar"
prefix = "/custom"
parallel_downloads = 16

[cache]
max_size = "10GB"
formula_ttl = 43200
download_ttl = 1209600
"#;

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.index.base_url, "https://custom.example.com");
    assert!(!config.index.auto_update);
    assert_eq!(config.index.update_interval, 3600);
    assert_eq!(config.install.parallel_downloads, 16);
    assert_eq!(config.cache.max_size, "10GB");
}

#[test]
fn test_config_partial_toml_uses_defaults() {
    let toml_str = r#"
[index]
auto_update = false
"#;

    let config: Config = toml::from_str(toml_str).unwrap();
    // Specified value
    assert!(!config.index.auto_update);
    // Should use defaults for unspecified values
    assert!(config.index.base_url.contains("neul-labs/stout-index"));
    assert_eq!(config.install.parallel_downloads, 4);
}

// ============================================================================
// InstalledPackages tests
// ============================================================================

#[test]
fn test_installed_packages_default() {
    let installed = InstalledPackages::default();
    assert_eq!(installed.count(), 0);
}

#[test]
fn test_installed_packages_add() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);

    assert_eq!(installed.count(), 1);
    assert!(installed.is_installed("wget"));
    assert!(!installed.is_installed("curl"));
}

#[test]
fn test_installed_packages_get() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 1, true);

    let pkg = installed.get("wget").unwrap();
    assert_eq!(pkg.version, "1.24.5");
    assert_eq!(pkg.revision, 1);
    assert!(pkg.requested);
    assert_eq!(pkg.installed_by, "stout");
}

#[test]
fn test_installed_packages_remove() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);
    installed.add("curl", "8.0.0", 0, false);

    let removed = installed.remove("wget");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().version, "1.24.5");

    assert!(!installed.is_installed("wget"));
    assert!(installed.is_installed("curl"));
}

#[test]
fn test_installed_packages_remove_nonexistent() {
    let mut installed = InstalledPackages::default();
    let removed = installed.remove("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_installed_packages_version_check() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);

    assert!(installed.is_version_installed("wget", "1.24.5"));
    assert!(!installed.is_version_installed("wget", "1.24.4"));
    assert!(!installed.is_version_installed("curl", "8.0.0"));
}

#[test]
fn test_installed_packages_names() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);
    installed.add("curl", "8.0.0", 0, false);
    installed.add("jq", "1.7", 0, true);

    let names: Vec<_> = installed.names().collect();
    assert_eq!(names.len(), 3);
    assert!(names.iter().any(|n| *n == "wget"));
    assert!(names.iter().any(|n| *n == "curl"));
    assert!(names.iter().any(|n| *n == "jq"));
}

#[test]
fn test_installed_packages_requested() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);  // requested
    installed.add("openssl", "3.0", 0, false); // dependency
    installed.add("jq", "1.7", 0, true);       // requested

    let requested: Vec<_> = installed.requested().collect();
    assert_eq!(requested.len(), 2);
    assert!(requested.iter().any(|(n, _)| *n == "wget"));
    assert!(requested.iter().any(|(n, _)| *n == "jq"));
}

#[test]
fn test_installed_packages_dependencies() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);  // requested
    installed.add("openssl", "3.0", 0, false); // dependency
    installed.add("zlib", "1.3", 0, false);    // dependency

    let deps: Vec<_> = installed.dependencies().collect();
    assert_eq!(deps.len(), 2);
    assert!(deps.iter().any(|(n, _)| *n == "openssl"));
    assert!(deps.iter().any(|(n, _)| *n == "zlib"));
}

#[test]
fn test_installed_packages_save_and_load() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 1, true);
    installed.add("curl", "8.0.0", 0, false);

    installed.save(&paths).unwrap();

    let loaded = InstalledPackages::load(&paths).unwrap();
    assert_eq!(loaded.count(), 2);
    assert!(loaded.is_installed("wget"));
    assert!(loaded.is_installed("curl"));
    assert_eq!(loaded.get("wget").unwrap().revision, 1);
}

#[test]
fn test_installed_packages_load_nonexistent_returns_empty() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let installed = InstalledPackages::load(&paths).unwrap();
    assert_eq!(installed.count(), 0);
}

#[test]
fn test_installed_package_has_timestamp() {
    let mut installed = InstalledPackages::default();
    installed.add("wget", "1.24.5", 0, true);

    let pkg = installed.get("wget").unwrap();
    // Should have a valid ISO 8601-ish timestamp
    assert!(pkg.installed_at.contains("-"));
    assert!(pkg.installed_at.contains("T"));
    assert!(pkg.installed_at.contains(":"));
}

// ============================================================================
// Paths tests
// ============================================================================

#[test]
fn test_paths_new() {
    let tmp = tempdir().unwrap();
    let stout_dir = tmp.path().join("stout");
    let prefix = tmp.path().join("homebrew");

    let paths = Paths::new(stout_dir.clone(), prefix.clone());

    assert_eq!(paths.stout_dir, stout_dir);
    assert_eq!(paths.prefix, prefix);
    assert_eq!(paths.cellar, prefix.join("Cellar"));
}

#[test]
fn test_paths_config_file() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    assert_eq!(paths.config_file(), tmp.path().join("config.toml"));
}

#[test]
fn test_paths_index_db() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    assert_eq!(paths.index_db(), tmp.path().join("index.db"));
}

#[test]
fn test_paths_manifest() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    assert_eq!(paths.manifest(), tmp.path().join("manifest.json"));
}

#[test]
fn test_paths_installed_file() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    assert_eq!(paths.installed_file(), tmp.path().join("state").join("installed.toml"));
}

#[test]
fn test_paths_formula_cache() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let expected = tmp.path().join("cache").join("formulas");
    assert_eq!(paths.formula_cache(), expected);
}

#[test]
fn test_paths_download_cache() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    let expected = tmp.path().join("cache").join("downloads");
    assert_eq!(paths.download_cache(), expected);
}

#[test]
fn test_paths_ensure_dirs() {
    let tmp = tempdir().unwrap();
    let paths = Paths::new(tmp.path().to_path_buf(), tmp.path().join("prefix"));

    paths.ensure_dirs().unwrap();

    assert!(tmp.path().exists());
    assert!(tmp.path().join("state").exists());
    assert!(tmp.path().join("cache").join("formulas").exists());
    assert!(tmp.path().join("cache").join("downloads").exists());
}

#[test]
fn test_paths_package_path() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("homebrew");
    let paths = Paths::new(tmp.path().to_path_buf(), prefix.clone());

    let pkg_path = paths.package_path("wget", "1.24.5");
    assert_eq!(pkg_path, prefix.join("Cellar").join("wget").join("1.24.5"));
}

#[test]
fn test_paths_is_installed() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("homebrew");
    let paths = Paths::new(tmp.path().to_path_buf(), prefix);

    // Package doesn't exist
    assert!(!paths.is_installed("wget", "1.24.5"));

    // Create the package directory
    let pkg_path = paths.package_path("wget", "1.24.5");
    std::fs::create_dir_all(&pkg_path).unwrap();

    assert!(paths.is_installed("wget", "1.24.5"));
    assert!(!paths.is_installed("wget", "1.24.4")); // Different version
    assert!(!paths.is_installed("curl", "8.0.0")); // Different package
}

#[test]
fn test_paths_installed_versions_empty() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("homebrew");
    let paths = Paths::new(tmp.path().to_path_buf(), prefix);

    let versions = paths.installed_versions("wget");
    assert!(versions.is_empty());
}

#[test]
fn test_paths_installed_versions() {
    let tmp = tempdir().unwrap();
    let prefix = tmp.path().join("homebrew");
    let paths = Paths::new(tmp.path().to_path_buf(), prefix);

    // Create multiple versions
    std::fs::create_dir_all(paths.package_path("wget", "1.24.4")).unwrap();
    std::fs::create_dir_all(paths.package_path("wget", "1.24.5")).unwrap();

    let versions = paths.installed_versions("wget");
    assert_eq!(versions.len(), 2);
    assert!(versions.contains(&"1.24.4".to_string()));
    assert!(versions.contains(&"1.24.5".to_string()));
}

// ============================================================================
// Error tests
// ============================================================================

#[test]
fn test_error_display_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: Error = io_err.into();
    assert!(err.to_string().contains("IO error"));
}

#[test]
fn test_error_display_config_not_found() {
    let err = Error::ConfigNotFound("/path/to/config.toml".to_string());
    assert!(err.to_string().contains("Config not found"));
}

#[test]
fn test_error_display_invalid_config() {
    let err = Error::InvalidConfig("missing required field".to_string());
    assert!(err.to_string().contains("Invalid config"));
}
