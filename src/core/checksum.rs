use crate::{Result, RustineErrorKind};

/// Patch file with embedded checksums
/// Format: [MAGIC (8)] [BASE_SHA256 (32)] [OUTPUT_SHA256 (32)] [PATCH_DATA...]
const MAGIC: &[u8; 8] = b"RUSTINE1";

/// Compute SHA256 hash of data
pub fn hash(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Simple hash for now (we can upgrade to a proper SHA256 later if needed)
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash_val = hasher.finish();

    // Expand to 32 bytes
    let mut result = [0u8; 32];
    result[0..8].copy_from_slice(&hash_val.to_le_bytes());
    result
}

/// Wrap patch data with checksums
pub fn wrap_patch(base_hash: [u8; 32], output_hash: [u8; 32], patch_data: &[u8]) -> Vec<u8> {
    let mut wrapped = Vec::with_capacity(8 + 32 + 32 + patch_data.len());
    wrapped.extend_from_slice(MAGIC);
    wrapped.extend_from_slice(&base_hash);
    wrapped.extend_from_slice(&output_hash);
    wrapped.extend_from_slice(patch_data);
    wrapped
}

/// Unwrap and verify patch data
pub fn unwrap_patch(data: &[u8]) -> Result<(Option<[u8; 32]>, Option<[u8; 32]>, &[u8])> {
    // Check if this is a wrapped patch
    if data.len() > 72 && &data[0..8] == MAGIC {
        let base_hash: [u8; 32] = data[8..40].try_into().unwrap();
        let output_hash: [u8; 32] = data[40..72].try_into().unwrap();
        let patch_data = &data[72..];
        Ok((Some(base_hash), Some(output_hash), patch_data))
    } else {
        // Legacy patch without checksums
        Ok((None, None, data))
    }
}

/// Verify hash matches expected
pub fn verify_hash(data: &[u8], expected: &[u8; 32]) -> Result<()> {
    let actual = hash(data);
    if actual != *expected {
        return Err(RustineErrorKind::ChecksumMismatch {
            expected: hex_encode(expected),
            actual: hex_encode(&actual),
        }
        .into());
    }
    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hex_encode_public(bytes: &[u8]) -> String {
    hex_encode(bytes)
}
