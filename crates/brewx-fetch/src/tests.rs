//! Tests for brewx-fetch

use crate::cache::DownloadCache;
use crate::client::BottleSpec;
use crate::error::Error;
use crate::progress::ProgressReporter;
use crate::verify::{sha256_bytes, verify_sha256};
use std::io::Write;
use tempfile::tempdir;

// ============================================================================
// DownloadCache tests
// ============================================================================

#[test]
fn test_cache_new() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());
    assert_eq!(cache.cache_dir(), tmp.path());
}

#[test]
fn test_cache_bottle_path() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    let path = cache.bottle_path("wget", "1.24.5", "x86_64_linux");
    assert!(path.to_string_lossy().contains("wget-1.24.5-x86_64_linux.tar.gz"));
    assert!(path.to_string_lossy().contains("downloads"));
}

#[test]
fn test_cache_has_bottle_empty() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    assert!(!cache.has_bottle("wget", "1.0.0", "x86_64_linux"));
}

#[test]
fn test_cache_store_and_get() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    // Store a bottle
    let data = b"test bottle content";
    let path = cache.store_bottle("wget", "1.0.0", "x86_64_linux", data).unwrap();

    assert!(path.exists());
    assert!(cache.has_bottle("wget", "1.0.0", "x86_64_linux"));

    // Get it back
    let got = cache.get_bottle("wget", "1.0.0", "x86_64_linux");
    assert!(got.is_some());
    assert_eq!(got.unwrap(), path);

    // Verify contents
    let contents = std::fs::read(&path).unwrap();
    assert_eq!(contents, data);
}

#[test]
fn test_cache_get_missing() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    let got = cache.get_bottle("nonexistent", "1.0.0", "x86_64_linux");
    assert!(got.is_none());
}

#[test]
fn test_cache_remove_bottle() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    // Store then remove
    cache.store_bottle("wget", "1.0.0", "x86_64_linux", b"test").unwrap();
    assert!(cache.has_bottle("wget", "1.0.0", "x86_64_linux"));

    cache.remove_bottle("wget", "1.0.0", "x86_64_linux").unwrap();
    assert!(!cache.has_bottle("wget", "1.0.0", "x86_64_linux"));
}

#[test]
fn test_cache_remove_nonexistent() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    // Should not fail when removing non-existent bottle
    let result = cache.remove_bottle("nonexistent", "1.0.0", "x86_64_linux");
    assert!(result.is_ok());
}

#[test]
fn test_cache_size_empty() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    let size = cache.cache_size().unwrap();
    assert_eq!(size, 0);
}

#[test]
fn test_cache_size_with_files() {
    let tmp = tempdir().unwrap();
    let cache = DownloadCache::new(tmp.path());

    let data1 = b"test content 1";
    let data2 = b"test content 2 longer";

    cache.store_bottle("pkg1", "1.0.0", "linux", data1).unwrap();
    cache.store_bottle("pkg2", "1.0.0", "linux", data2).unwrap();

    let size = cache.cache_size().unwrap();
    assert_eq!(size, (data1.len() + data2.len()) as u64);
}

// ============================================================================
// Verify tests
// ============================================================================

#[test]
fn test_sha256_bytes_known_value() {
    // Known SHA256 hash
    let hash = sha256_bytes(b"hello world");
    assert_eq!(
        hash,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn test_sha256_bytes_empty() {
    let hash = sha256_bytes(b"");
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn test_sha256_bytes_large() {
    // Test with larger data
    let data = vec![0u8; 1024 * 1024]; // 1MB of zeros
    let hash = sha256_bytes(&data);
    assert_eq!(hash.len(), 64); // SHA256 produces 64 hex chars
}

#[test]
fn test_verify_sha256_success() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("test.txt");

    let data = b"test file content";
    std::fs::write(&path, data).unwrap();

    let expected_hash = sha256_bytes(data);
    let result = verify_sha256(&path, &expected_hash);
    assert!(result.is_ok());
}

#[test]
fn test_verify_sha256_mismatch() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("test.txt");

    std::fs::write(&path, b"actual content").unwrap();

    let result = verify_sha256(&path, "wrong_hash");
    assert!(matches!(result, Err(Error::ChecksumMismatch { .. })));
}

#[test]
fn test_verify_sha256_file_not_found() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("nonexistent.txt");

    let result = verify_sha256(&path, "any_hash");
    assert!(matches!(result, Err(Error::Io(_))));
}

// ============================================================================
// BottleSpec tests
// ============================================================================

#[test]
fn test_bottle_spec_creation() {
    let spec = BottleSpec {
        name: "wget".to_string(),
        version: "1.24.5".to_string(),
        platform: "arm64_sonoma".to_string(),
        url: "https://ghcr.io/v2/homebrew/core/wget/blobs/sha256:abc123".to_string(),
        sha256: "abc123def456".to_string(),
    };

    assert_eq!(spec.name, "wget");
    assert_eq!(spec.version, "1.24.5");
    assert_eq!(spec.platform, "arm64_sonoma");
    assert!(spec.url.contains("ghcr.io"));
    assert_eq!(spec.sha256, "abc123def456");
}

#[test]
fn test_bottle_spec_clone() {
    let spec = BottleSpec {
        name: "curl".to_string(),
        version: "8.0.0".to_string(),
        platform: "x86_64_linux".to_string(),
        url: "https://example.com/bottle.tar.gz".to_string(),
        sha256: "deadbeef".to_string(),
    };

    let cloned = spec.clone();
    assert_eq!(cloned.name, spec.name);
    assert_eq!(cloned.version, spec.version);
}

// ============================================================================
// ProgressReporter tests
// ============================================================================

#[test]
fn test_progress_reporter_new() {
    let reporter = ProgressReporter::new();
    // Just verify it doesn't panic
    let _ = reporter;
}

#[test]
fn test_progress_reporter_default() {
    let reporter = ProgressReporter::default();
    let _ = reporter;
}

#[test]
fn test_progress_download_bar() {
    let reporter = ProgressReporter::new();
    let progress = reporter.new_download("wget", 1024 * 1024);

    // Test operations don't panic
    progress.set_position(512 * 1024);
    progress.inc(1024);
    progress.set_message("Downloading...");
    progress.finish();
}

#[test]
fn test_progress_spinner() {
    let reporter = ProgressReporter::new();
    let progress = reporter.new_spinner("Resolving dependencies...");

    progress.set_message("Almost done...");
    progress.finish_with_message("Done!");
}

#[test]
fn test_progress_summary() {
    let reporter = ProgressReporter::new();
    let progress = reporter.new_summary(10, "Installing packages");

    progress.set_position(5);
    progress.inc(1);
    progress.finish();
}

// ============================================================================
// Error tests
// ============================================================================

#[test]
fn test_error_display_checksum() {
    let err = Error::ChecksumMismatch {
        path: "/path/to/file".to_string(),
        expected: "expected_hash".to_string(),
        actual: "actual_hash".to_string(),
    };

    let msg = err.to_string();
    assert!(msg.contains("Checksum mismatch"));
    assert!(msg.contains("/path/to/file"));
    assert!(msg.contains("expected_hash"));
    assert!(msg.contains("actual_hash"));
}

#[test]
fn test_error_display_download_failed() {
    let err = Error::DownloadFailed("Connection timeout".to_string());
    assert_eq!(err.to_string(), "Download failed: Connection timeout");
}

#[test]
fn test_error_display_not_in_cache() {
    let err = Error::NotInCache("wget-1.0.0".to_string());
    assert_eq!(err.to_string(), "File not found in cache: wget-1.0.0");
}

#[test]
fn test_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err: Error = io_err.into();
    assert!(matches!(err, Error::Io(_)));
}
