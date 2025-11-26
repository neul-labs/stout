//! Tests for brewx-install

use crate::error::Error;
use crate::extract::remove_package;
use crate::link::{link_package, unlink_package};
use crate::receipt::{read_receipt, write_receipt, InstallReceipt, RuntimeDependency};
use std::os::unix::fs::symlink;
use tempfile::tempdir;

// ============================================================================
// InstallReceipt tests
// ============================================================================

#[test]
fn test_receipt_new_bottle() {
    let receipt = InstallReceipt::new_bottle(
        "homebrew/core",
        true,
        vec![RuntimeDependency {
            full_name: "openssl@3".to_string(),
            version: "3.1.0".to_string(),
            revision: Some(1),
        }],
    );

    assert_eq!(receipt.source.tap, "homebrew/core");
    assert!(receipt.installed_on_request);
    assert!(!receipt.installed_as_dependency);
    assert!(receipt.poured_from_bottle);
    assert!(receipt.install_time > 0);
    assert_eq!(receipt.runtime_dependencies.len(), 1);
    assert_eq!(receipt.runtime_dependencies[0].full_name, "openssl@3");
}

#[test]
fn test_receipt_as_dependency() {
    let receipt = InstallReceipt::new_bottle("homebrew/core", false, vec![]);

    assert!(!receipt.installed_on_request);
    assert!(receipt.installed_as_dependency);
}

#[test]
fn test_receipt_write_and_read() {
    let tmp = tempdir().unwrap();

    let original = InstallReceipt::new_bottle(
        "homebrew/core",
        true,
        vec![
            RuntimeDependency {
                full_name: "openssl@3".to_string(),
                version: "3.1.0".to_string(),
                revision: None,
            },
            RuntimeDependency {
                full_name: "ca-certificates".to_string(),
                version: "2024.01.01".to_string(),
                revision: Some(0),
            },
        ],
    );

    write_receipt(tmp.path(), &original).unwrap();

    let loaded = read_receipt(tmp.path()).unwrap();
    assert_eq!(loaded.source.tap, original.source.tap);
    assert_eq!(loaded.installed_on_request, original.installed_on_request);
    assert_eq!(loaded.runtime_dependencies.len(), 2);
    assert!(loaded.poured_from_bottle);
}

#[test]
fn test_receipt_serialization_format() {
    let receipt = InstallReceipt::new_bottle("homebrew/core", true, vec![]);
    let json = serde_json::to_string_pretty(&receipt).unwrap();

    // Verify JSON contains expected fields
    assert!(json.contains("homebrew_version"));
    assert!(json.contains("installed_as_dependency"));
    assert!(json.contains("installed_on_request"));
    assert!(json.contains("install_time"));
    assert!(json.contains("poured_from_bottle"));
    assert!(json.contains("homebrew/core"));
}

#[test]
fn test_receipt_read_nonexistent() {
    let tmp = tempdir().unwrap();
    let result = read_receipt(tmp.path());
    assert!(matches!(result, Err(Error::Io(_))));
}

#[test]
fn test_receipt_read_invalid_json() {
    let tmp = tempdir().unwrap();
    let receipt_path = tmp.path().join("INSTALL_RECEIPT.json");
    std::fs::write(&receipt_path, "not valid json").unwrap();

    let result = read_receipt(tmp.path());
    assert!(matches!(result, Err(Error::Json(_))));
}

// ============================================================================
// RuntimeDependency tests
// ============================================================================

#[test]
fn test_runtime_dependency_serialization() {
    let dep = RuntimeDependency {
        full_name: "zlib".to_string(),
        version: "1.3".to_string(),
        revision: Some(2),
    };

    let json = serde_json::to_string(&dep).unwrap();
    assert!(json.contains("zlib"));
    assert!(json.contains("1.3"));
    assert!(json.contains("revision"));

    let parsed: RuntimeDependency = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.full_name, "zlib");
    assert_eq!(parsed.version, "1.3");
    assert_eq!(parsed.revision, Some(2));
}

#[test]
fn test_runtime_dependency_no_revision() {
    let dep = RuntimeDependency {
        full_name: "zlib".to_string(),
        version: "1.3".to_string(),
        revision: None,
    };

    let json = serde_json::to_string(&dep).unwrap();
    // revision should be skipped when None
    assert!(!json.contains("revision"));
}

// ============================================================================
// remove_package tests
// ============================================================================

#[test]
fn test_remove_package() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");

    // Create a fake installed package
    let pkg_path = cellar.join("wget").join("1.24.5");
    let bin_dir = pkg_path.join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("wget"), "#!/bin/bash").unwrap();

    // Remove it
    remove_package(&cellar, "wget", "1.24.5").unwrap();

    // Verify it's gone
    assert!(!pkg_path.exists());
    // Parent should also be removed since it's empty
    assert!(!cellar.join("wget").exists());
}

#[test]
fn test_remove_package_not_found() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    std::fs::create_dir_all(&cellar).unwrap();

    let result = remove_package(&cellar, "nonexistent", "1.0.0");
    assert!(matches!(result, Err(Error::PackageNotFound(_))));
}

#[test]
fn test_remove_package_keeps_other_versions() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");

    // Create two versions
    let v1_path = cellar.join("wget").join("1.24.4");
    let v2_path = cellar.join("wget").join("1.24.5");
    std::fs::create_dir_all(&v1_path).unwrap();
    std::fs::create_dir_all(&v2_path).unwrap();

    // Remove one version
    remove_package(&cellar, "wget", "1.24.5").unwrap();

    // Other version should still exist
    assert!(v1_path.exists());
    assert!(!v2_path.exists());
    // Parent should still exist since v1 is there
    assert!(cellar.join("wget").exists());
}

// ============================================================================
// link_package / unlink_package tests
// ============================================================================

#[test]
fn test_link_package_creates_symlinks() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    // Create a fake installed package with bin
    let pkg_path = cellar.join("wget").join("1.24.5");
    let bin_dir = pkg_path.join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    std::fs::write(bin_dir.join("wget"), "#!/bin/bash\necho wget").unwrap();

    // Link it
    let linked = link_package(&pkg_path, &prefix).unwrap();

    // Should have created symlinks
    assert!(!linked.is_empty());

    // bin/wget should exist and be a symlink
    let linked_binary = prefix.join("bin").join("wget");
    assert!(linked_binary.symlink_metadata().is_ok());
    assert!(linked_binary.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn test_link_package_creates_opt_link() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    let pkg_path = cellar.join("wget").join("1.24.5");
    std::fs::create_dir_all(&pkg_path).unwrap();

    link_package(&pkg_path, &prefix).unwrap();

    // opt/wget should exist and be a symlink
    let opt_link = prefix.join("opt").join("wget");
    assert!(opt_link.symlink_metadata().is_ok());
    assert!(opt_link.symlink_metadata().unwrap().file_type().is_symlink());
}

#[test]
fn test_link_multiple_dirs() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    let pkg_path = cellar.join("mypackage").join("1.0.0");

    // Create bin, lib, and include directories
    std::fs::create_dir_all(pkg_path.join("bin")).unwrap();
    std::fs::create_dir_all(pkg_path.join("lib")).unwrap();
    std::fs::create_dir_all(pkg_path.join("include")).unwrap();
    std::fs::write(pkg_path.join("bin").join("mybin"), "binary").unwrap();
    std::fs::write(pkg_path.join("lib").join("libmy.so"), "library").unwrap();
    std::fs::write(pkg_path.join("include").join("my.h"), "header").unwrap();

    let linked = link_package(&pkg_path, &prefix).unwrap();

    // Should have linked files from all directories (plus opt link)
    assert!(linked.len() >= 4);
    assert!(prefix.join("bin").join("mybin").exists());
    assert!(prefix.join("lib").join("libmy.so").exists());
    assert!(prefix.join("include").join("my.h").exists());
}

#[test]
fn test_unlink_package() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    let pkg_path = cellar.join("wget").join("1.24.5");
    std::fs::create_dir_all(pkg_path.join("bin")).unwrap();
    std::fs::write(pkg_path.join("bin").join("wget"), "binary").unwrap();

    // Link then unlink
    link_package(&pkg_path, &prefix).unwrap();

    let bin_link = prefix.join("bin").join("wget");
    assert!(bin_link.symlink_metadata().is_ok());

    let unlinked = unlink_package(&pkg_path, &prefix).unwrap();
    assert!(!unlinked.is_empty());

    // Symlinks should be gone
    assert!(!bin_link.exists());
    assert!(bin_link.symlink_metadata().is_err());
}

#[test]
fn test_link_skips_existing_non_symlinks() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    let pkg_path = cellar.join("wget").join("1.24.5");
    std::fs::create_dir_all(pkg_path.join("bin")).unwrap();
    std::fs::write(pkg_path.join("bin").join("wget"), "binary").unwrap();

    // Create a real file at the target location
    std::fs::create_dir_all(prefix.join("bin")).unwrap();
    std::fs::write(prefix.join("bin").join("wget"), "existing binary").unwrap();

    // Link should succeed but skip the existing file
    let linked = link_package(&pkg_path, &prefix).unwrap();

    // bin/wget should not be in linked (was skipped) - check for the full path
    let bin_wget = prefix.join("bin").join("wget");
    assert!(!linked.contains(&bin_wget));

    // Original file should still have its content
    let content = std::fs::read_to_string(&bin_wget).unwrap();
    assert_eq!(content, "existing binary");
}

#[test]
fn test_link_empty_package() {
    let tmp = tempdir().unwrap();
    let cellar = tmp.path().join("Cellar");
    let prefix = tmp.path().join("prefix");

    let pkg_path = cellar.join("empty").join("1.0.0");
    std::fs::create_dir_all(&pkg_path).unwrap();

    // Should succeed but return opt link only
    let linked = link_package(&pkg_path, &prefix).unwrap();
    assert!(linked.iter().any(|p| p.to_string_lossy().contains("opt")));
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
fn test_error_display_package_not_found() {
    let err = Error::PackageNotFound("wget/1.0.0".to_string());
    assert_eq!(err.to_string(), "Package not found: wget/1.0.0");
}

#[test]
fn test_error_display_link_failed() {
    let err = Error::LinkFailed("Could not create symlink".to_string());
    assert!(err.to_string().contains("Link failed"));
}

#[test]
fn test_error_display_invalid_bottle() {
    let err = Error::InvalidBottle("Missing version directory".to_string());
    assert!(err.to_string().contains("Invalid bottle"));
}

#[test]
fn test_error_display_archive() {
    let err = Error::Archive("Corrupted tarball".to_string());
    assert!(err.to_string().contains("Archive error"));
}
