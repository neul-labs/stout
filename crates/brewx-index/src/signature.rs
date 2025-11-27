//! Ed25519 signature verification for index integrity
//!
//! This module provides cryptographic verification of index data using Ed25519 signatures.
//! The signing flow is:
//! 1. Index server signs the index manifest with its private key
//! 2. Client downloads manifest + signature
//! 3. Client verifies signature using trusted public key
//! 4. If valid, index data is trusted

use crate::error::{Error, Result};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::{debug, info, warn};

/// Default public key for brewx-index (neul-labs/brewx-index)
/// This key is used to verify the integrity and authenticity of index updates.
/// The corresponding private key is kept secure in GitHub Secrets.
pub const DEFAULT_PUBLIC_KEY_HEX: &str =
    "e58d628836f72ecc7f6964ba2b70523d7c1c46512441ef8eccf2fa55ad0258f2";

/// Signed manifest containing index metadata and signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    /// Version of the manifest format
    pub version: u32,
    /// SHA-256 hash of the index database
    pub index_sha256: String,
    /// Timestamp when signed (Unix epoch seconds)
    pub signed_at: u64,
    /// Index version/commit
    pub index_version: String,
    /// Formula count
    pub formula_count: usize,
    /// Cask count (optional)
    pub cask_count: Option<usize>,
    /// Ed25519 signature of the manifest data (hex-encoded)
    pub signature: String,
}

/// Verifier for Ed25519 signatures
pub struct SignatureVerifier {
    /// Trusted public keys (can have multiple for key rotation)
    public_keys: Vec<VerifyingKey>,
}

impl SignatureVerifier {
    /// Create a new verifier with the default Homebrew public key
    pub fn new() -> Result<Self> {
        Self::with_public_key(DEFAULT_PUBLIC_KEY_HEX)
    }

    /// Create a verifier with a specific public key (hex-encoded)
    pub fn with_public_key(public_key_hex: &str) -> Result<Self> {
        let key = parse_public_key(public_key_hex)?;
        Ok(Self {
            public_keys: vec![key],
        })
    }

    /// Create a verifier with multiple trusted public keys
    pub fn with_public_keys(public_keys_hex: &[&str]) -> Result<Self> {
        let mut keys = Vec::with_capacity(public_keys_hex.len());
        for key_hex in public_keys_hex {
            keys.push(parse_public_key(key_hex)?);
        }
        Ok(Self { public_keys: keys })
    }

    /// Add a trusted public key
    pub fn add_public_key(&mut self, public_key_hex: &str) -> Result<()> {
        let key = parse_public_key(public_key_hex)?;
        self.public_keys.push(key);
        Ok(())
    }

    /// Get reference to the public keys for direct verification
    pub fn public_keys(&self) -> &[VerifyingKey] {
        &self.public_keys
    }

    /// Verify a signed manifest
    ///
    /// Returns true if the manifest signature is valid for any of the trusted keys
    pub fn verify_manifest(&self, manifest: &SignedManifest) -> Result<bool> {
        // Reconstruct the signed data (everything except the signature)
        let signed_data = format!(
            "brewx-index:v{}:{}:{}:{}:{}:{}",
            manifest.version,
            manifest.index_sha256,
            manifest.signed_at,
            manifest.index_version,
            manifest.formula_count,
            manifest.cask_count.unwrap_or(0)
        );

        debug!("Verifying signature for manifest");

        // Parse the signature
        let signature_bytes = hex::decode(&manifest.signature).map_err(|e| {
            Error::SignatureInvalid(format!("Invalid signature hex: {}", e))
        })?;

        let signature = Signature::from_slice(&signature_bytes).map_err(|e| {
            Error::SignatureInvalid(format!("Invalid signature format: {}", e))
        })?;

        // Try each public key
        for key in &self.public_keys {
            if key.verify(signed_data.as_bytes(), &signature).is_ok() {
                info!("Manifest signature verified successfully");
                return Ok(true);
            }
        }

        warn!("Manifest signature verification failed");
        Ok(false)
    }

    /// Verify an index file against a manifest
    ///
    /// Checks that the file's SHA-256 matches the manifest's recorded hash
    pub fn verify_index_file(&self, manifest: &SignedManifest, index_path: &Path) -> Result<bool> {
        let file_hash = compute_file_sha256(index_path)?;

        if file_hash == manifest.index_sha256 {
            debug!("Index file hash matches manifest");
            Ok(true)
        } else {
            warn!(
                "Index file hash mismatch: expected {}, got {}",
                manifest.index_sha256, file_hash
            );
            Ok(false)
        }
    }

    /// Full verification: check both signature and file hash
    pub fn verify_full(
        &self,
        manifest: &SignedManifest,
        index_path: &Path,
    ) -> Result<VerificationResult> {
        let signature_valid = self.verify_manifest(manifest)?;
        let hash_valid = self.verify_index_file(manifest, index_path)?;

        Ok(VerificationResult {
            signature_valid,
            hash_valid,
            index_version: manifest.index_version.clone(),
            formula_count: manifest.formula_count,
            signed_at: manifest.signed_at,
        })
    }
}

impl Default for SignatureVerifier {
    fn default() -> Self {
        Self::new().expect("Default verifier should be valid")
    }
}

/// Result of full verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether the Ed25519 signature is valid
    pub signature_valid: bool,
    /// Whether the index file hash matches
    pub hash_valid: bool,
    /// Index version from manifest
    pub index_version: String,
    /// Number of formulas in index
    pub formula_count: usize,
    /// When the manifest was signed
    pub signed_at: u64,
}

impl VerificationResult {
    /// Check if verification fully passed
    pub fn is_valid(&self) -> bool {
        self.signature_valid && self.hash_valid
    }
}

/// Parse a hex-encoded public key
fn parse_public_key(hex_key: &str) -> Result<VerifyingKey> {
    let key_bytes = hex::decode(hex_key).map_err(|e| {
        Error::SignatureInvalid(format!("Invalid public key hex: {}", e))
    })?;

    if key_bytes.len() != 32 {
        return Err(Error::SignatureInvalid(format!(
            "Public key must be 32 bytes, got {}",
            key_bytes.len()
        )));
    }

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes);

    VerifyingKey::from_bytes(&key_array).map_err(|e| {
        Error::SignatureInvalid(format!("Invalid public key: {}", e))
    })
}

/// Compute SHA-256 hash of a file
pub fn compute_file_sha256(path: &Path) -> Result<String> {
    let data = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let hash = hasher.finalize();
    Ok(hex::encode(hash))
}

/// Compute SHA-256 hash of data
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let hash = hasher.finalize();
    hex::encode(hash)
}

/// Trusted key configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedKeys {
    /// Primary public key (hex-encoded)
    pub primary: String,
    /// Additional trusted keys for rotation
    #[serde(default)]
    pub additional: Vec<String>,
    /// Minimum required signature age (to prevent replay attacks)
    #[serde(default)]
    pub max_age_seconds: Option<u64>,
}

impl Default for TrustedKeys {
    fn default() -> Self {
        Self {
            primary: DEFAULT_PUBLIC_KEY_HEX.to_string(),
            additional: vec![],
            max_age_seconds: Some(86400 * 7), // 7 days
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let hash = compute_sha256(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_verification_result() {
        let result = VerificationResult {
            signature_valid: true,
            hash_valid: true,
            index_version: "test".to_string(),
            formula_count: 100,
            signed_at: 0,
        };
        assert!(result.is_valid());

        let invalid = VerificationResult {
            signature_valid: false,
            hash_valid: true,
            index_version: "test".to_string(),
            formula_count: 100,
            signed_at: 0,
        };
        assert!(!invalid.is_valid());
    }
}
