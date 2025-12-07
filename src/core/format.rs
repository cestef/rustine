use crate::{Result, RustineErrorKind};

/// Magic bytes for rustine patch format v2
const MAGIC_V2: &[u8; 8] = b"RUSTINE2";

/// Magic bytes for legacy rustine patch format v1
const MAGIC_V1: &[u8; 8] = b"RUSTINE1";

/// Current format version
const VERSION: u8 = 2;

/// Feature flags
pub const FLAG_BASE_CHECKSUM: u32 = 1 << 0; // 0x00000001
pub const FLAG_OUTPUT_CHECKSUM: u32 = 1 << 1; // 0x00000002
pub const FLAG_REVERSE_PATCH: u32 = 1 << 2; // 0x00000004
pub const FLAG_METADATA: u32 = 1 << 3; // 0x00000008

/// Patch data with all optional features
#[derive(Debug)]
pub struct PatchData {
    pub base_checksum: Option<[u8; 32]>,
    pub output_checksum: Option<[u8; 32]>,
    pub forward_patch: Vec<u8>,
    pub reverse_patch: Option<Vec<u8>>,
    pub metadata: Option<String>,
}

impl PatchData {
    /// Create a new patch with just forward data
    pub fn new(forward_patch: Vec<u8>) -> Self {
        Self {
            base_checksum: None,
            output_checksum: None,
            forward_patch,
            reverse_patch: None,
            metadata: None,
        }
    }

    /// Add checksums
    pub fn with_checksums(mut self, base: [u8; 32], output: [u8; 32]) -> Self {
        self.base_checksum = Some(base);
        self.output_checksum = Some(output);
        self
    }

    /// Add reverse patch
    pub fn with_reverse(mut self, reverse: Vec<u8>) -> Self {
        self.reverse_patch = Some(reverse);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: String) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Serialize to bytes
    pub fn serialize(&self) -> Vec<u8> {
        let mut flags = 0u32;
        let mut size = 13; // magic(8) + version(1) + flags(4)

        // Calculate size and set flags
        if self.base_checksum.is_some() {
            flags |= FLAG_BASE_CHECKSUM;
            size += 32;
        }
        if self.output_checksum.is_some() {
            flags |= FLAG_OUTPUT_CHECKSUM;
            size += 32;
        }
        if let Some(meta) = &self.metadata {
            flags |= FLAG_METADATA;
            size += 4 + meta.len();
        }
        size += 8 + self.forward_patch.len(); // forward_len(8) + data
        if let Some(rev) = &self.reverse_patch {
            flags |= FLAG_REVERSE_PATCH;
            size += 8 + rev.len(); // reverse_len(8) + data
        }

        let mut data = Vec::with_capacity(size);

        // Write header
        data.extend_from_slice(MAGIC_V2);
        data.push(VERSION);
        data.extend_from_slice(&flags.to_le_bytes());

        // Write optional checksums
        if let Some(hash) = self.base_checksum {
            data.extend_from_slice(&hash);
        }
        if let Some(hash) = self.output_checksum {
            data.extend_from_slice(&hash);
        }

        // Write optional metadata
        if let Some(meta) = &self.metadata {
            data.extend_from_slice(&(meta.len() as u32).to_le_bytes());
            data.extend_from_slice(meta.as_bytes());
        }

        // Write forward patch
        data.extend_from_slice(&(self.forward_patch.len() as u64).to_le_bytes());
        data.extend_from_slice(&self.forward_patch);

        // Write reverse patch if present
        if let Some(rev) = &self.reverse_patch {
            data.extend_from_slice(&(rev.len() as u64).to_le_bytes());
            data.extend_from_slice(rev);
        }

        data
    }

    /// Deserialize from bytes
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        // Check magic and determine version
        if data.len() < 13 {
            return Err(RustineErrorKind::CorruptedPatch {
                details: format!("file too small ({} bytes, expected at least 13)", data.len()),
            }
            .into());
        }

        // Check for v1 legacy format
        if &data[0..8] == MAGIC_V1 {
            return deserialize_v1(data);
        }

        // Check for v2 format
        if &data[0..8] != MAGIC_V2 {
            // Try to parse as raw bsdiff patch (no magic)
            return Ok(PatchData::new(data.to_vec()));
        }

        let version = data[8];
        if version != VERSION {
            return Err(RustineErrorKind::UnsupportedVersion { version }.into());
        }

        let flags = u32::from_le_bytes([data[9], data[10], data[11], data[12]]);
        let mut offset = 13;

        // Read optional checksums
        let base_checksum = if flags & FLAG_BASE_CHECKSUM != 0 {
            if data.len() < offset + 32 {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated base checksum".to_string(),
                }
                .into());
            }
            let hash: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
            offset += 32;
            Some(hash)
        } else {
            None
        };

        let output_checksum = if flags & FLAG_OUTPUT_CHECKSUM != 0 {
            if data.len() < offset + 32 {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated output checksum".to_string(),
                }
                .into());
            }
            let hash: [u8; 32] = data[offset..offset + 32].try_into().unwrap();
            offset += 32;
            Some(hash)
        } else {
            None
        };

        // Read optional metadata
        let metadata = if flags & FLAG_METADATA != 0 {
            if data.len() < offset + 4 {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated metadata length".to_string(),
                }
                .into());
            }
            let meta_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if data.len() < offset + meta_len {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated metadata".to_string(),
                }
                .into());
            }
            let meta = String::from_utf8_lossy(&data[offset..offset + meta_len]).to_string();
            offset += meta_len;
            Some(meta)
        } else {
            None
        };

        // Read forward patch
        if data.len() < offset + 8 {
            return Err(RustineErrorKind::CorruptedPatch {
                details: "truncated forward patch length".to_string(),
            }
            .into());
        }
        let forward_len = u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        offset += 8;

        if data.len() < offset + forward_len {
            return Err(RustineErrorKind::CorruptedPatch {
                details: "truncated forward patch data".to_string(),
            }
            .into());
        }
        let forward_patch = data[offset..offset + forward_len].to_vec();
        offset += forward_len;

        // Read reverse patch if present
        let reverse_patch = if flags & FLAG_REVERSE_PATCH != 0 {
            if data.len() < offset + 8 {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated reverse patch length".to_string(),
                }
                .into());
            }
            let reverse_len = u64::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
                data[offset + 4],
                data[offset + 5],
                data[offset + 6],
                data[offset + 7],
            ]) as usize;
            offset += 8;

            if data.len() < offset + reverse_len {
                return Err(RustineErrorKind::CorruptedPatch {
                    details: "truncated reverse patch data".to_string(),
                }
                .into());
            }
            let reverse = data[offset..offset + reverse_len].to_vec();
            Some(reverse)
        } else {
            None
        };

        Ok(PatchData {
            base_checksum,
            output_checksum,
            forward_patch,
            reverse_patch,
            metadata,
        })
    }
}

/// Deserialize legacy v1 format
fn deserialize_v1(data: &[u8]) -> Result<PatchData> {
    if data.len() < 72 {
        return Err(RustineErrorKind::CorruptedPatch {
            details: format!("v1 patch too small ({} bytes, expected at least 72)", data.len()),
        }
        .into());
    }

    let base_checksum: [u8; 32] = data[8..40].try_into().unwrap();
    let output_checksum: [u8; 32] = data[40..72].try_into().unwrap();
    let forward_patch = data[72..].to_vec();

    Ok(PatchData {
        base_checksum: Some(base_checksum),
        output_checksum: Some(output_checksum),
        forward_patch,
        reverse_patch: None,
        metadata: None,
    })
}

/// Compute SHA256-like hash of data (using DefaultHasher for simplicity)
pub fn hash(data: &[u8]) -> [u8; 32] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash_val = hasher.finish();

    // Expand to 32 bytes
    let mut result = [0u8; 32];
    result[0..8].copy_from_slice(&hash_val.to_le_bytes());
    result
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
