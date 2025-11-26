//! Checksum verification

use crate::error::{Error, Result};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::Path;

/// Verify a file's SHA256 checksum
pub fn verify_sha256(path: impl AsRef<Path>, expected: &str) -> Result<()> {
    let path = path.as_ref();
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    let actual = format!("{:x}", hasher.finalize());

    if actual != expected {
        return Err(Error::ChecksumMismatch {
            path: path.display().to_string(),
            expected: expected.to_string(),
            actual,
        });
    }

    Ok(())
}

/// Calculate SHA256 of bytes
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_bytes() {
        let hash = sha256_bytes(b"hello world");
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
}
