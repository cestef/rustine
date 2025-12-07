use crate::{Result, RustineErrorKind};

/// Size constants
const HASH_SIZE: usize = 32;
const U32_SIZE: usize = 4;
const U64_SIZE: usize = 8;
const RUSTINE2_HEADER_SIZE: usize = 13; // magic(8) + version(1) + flags(4)

/// Patch format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchFormat {
    /// RUSTINE2 format with optional features
    Rustine2,
    /// Raw BSDIFF4 patch
    Bsdiff4,
}

impl PatchFormat {
    /// Magic bytes for RUSTINE2 format
    const RUSTINE2_MAGIC: &'static [u8; 8] = b"RUSTINE2";

    /// Current RUSTINE2 version
    const RUSTINE2_VERSION: u8 = 2;

    /// Detect format from patch data
    pub fn detect(data: &[u8]) -> Self {
        if data.len() >= 8 && &data[0..8] == Self::RUSTINE2_MAGIC {
            Self::Rustine2
        } else {
            Self::Bsdiff4
        }
    }

    /// Get magic bytes for this format (if any)
    pub fn magic(&self) -> Option<&'static [u8; 8]> {
        match self {
            Self::Rustine2 => Some(Self::RUSTINE2_MAGIC),
            Self::Bsdiff4 => None,
        }
    }

    /// Get version number for this format (if any)
    pub fn version(&self) -> Option<u8> {
        match self {
            Self::Rustine2 => Some(Self::RUSTINE2_VERSION),
            Self::Bsdiff4 => None,
        }
    }

    /// Get format name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rustine2 => "RUSTINE2",
            Self::Bsdiff4 => "BSDIFF4",
        }
    }
}

/// Feature flags for RUSTINE2 format
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
        data.extend_from_slice(PatchFormat::Rustine2.magic().unwrap());
        data.push(PatchFormat::Rustine2.version().unwrap());
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
        let format = PatchFormat::detect(data);

        match format {
            PatchFormat::Rustine2 => {
                if data.len() < RUSTINE2_HEADER_SIZE {
                    return Err(RustineErrorKind::CorruptedPatch {
                        details: format!(
                            "file too small ({} bytes, expected at least {})",
                            data.len(),
                            RUSTINE2_HEADER_SIZE
                        ),
                    }
                    .into());
                }

                let version = data[8];
                if version != PatchFormat::Rustine2.version().unwrap() {
                    return Err(RustineErrorKind::UnsupportedVersion { version }.into());
                }

                deserialize_rustine2(data)
            }
            PatchFormat::Bsdiff4 => {
                // Raw BSDIFF4 patch
                Ok(PatchData::new(data.to_vec()))
            }
        }
    }
}

/// Helper to read fixed-size data and advance offset
fn read_bytes<const N: usize>(
    data: &[u8],
    offset: &mut usize,
    field_name: &str,
) -> Result<[u8; N]> {
    if data.len() < *offset + N {
        return Err(RustineErrorKind::CorruptedPatch {
            details: format!("truncated {}", field_name),
        }
        .into());
    }
    let bytes: [u8; N] = data[*offset..*offset + N].try_into().unwrap();
    *offset += N;
    Ok(bytes)
}

/// Helper to read u32 little-endian
fn read_u32_le(data: &[u8], offset: &mut usize, field_name: &str) -> Result<u32> {
    let bytes = read_bytes::<U32_SIZE>(data, offset, field_name)?;
    Ok(u32::from_le_bytes(bytes))
}

/// Helper to read u64 little-endian
fn read_u64_le(data: &[u8], offset: &mut usize, field_name: &str) -> Result<u64> {
    let bytes = read_bytes::<U64_SIZE>(data, offset, field_name)?;
    Ok(u64::from_le_bytes(bytes))
}

/// Helper to read variable-length data
fn read_var_bytes(
    data: &[u8],
    offset: &mut usize,
    len: usize,
    field_name: &str,
) -> Result<Vec<u8>> {
    if data.len() < *offset + len {
        return Err(RustineErrorKind::CorruptedPatch {
            details: format!("truncated {}", field_name),
        }
        .into());
    }
    let bytes = data[*offset..*offset + len].to_vec();
    *offset += len;
    Ok(bytes)
}

/// Deserialize RUSTINE2 format
fn deserialize_rustine2(data: &[u8]) -> Result<PatchData> {
    let flags = u32::from_le_bytes([data[9], data[10], data[11], data[12]]);
    let mut offset = RUSTINE2_HEADER_SIZE;

    // Read optional checksums
    let base_checksum = if flags & FLAG_BASE_CHECKSUM != 0 {
        Some(read_bytes::<HASH_SIZE>(data, &mut offset, "base checksum")?)
    } else {
        None
    };

    let output_checksum = if flags & FLAG_OUTPUT_CHECKSUM != 0 {
        Some(read_bytes::<HASH_SIZE>(
            data,
            &mut offset,
            "output checksum",
        )?)
    } else {
        None
    };

    // Read optional metadata
    let metadata = if flags & FLAG_METADATA != 0 {
        let meta_len = read_u32_le(data, &mut offset, "metadata length")? as usize;
        let meta_bytes = read_var_bytes(data, &mut offset, meta_len, "metadata")?;
        Some(String::from_utf8_lossy(&meta_bytes).to_string())
    } else {
        None
    };

    // Read forward patch
    let forward_len = read_u64_le(data, &mut offset, "forward patch length")? as usize;
    let forward_patch = read_var_bytes(data, &mut offset, forward_len, "forward patch data")?;

    // Read reverse patch if present
    let reverse_patch = if flags & FLAG_REVERSE_PATCH != 0 {
        let reverse_len = read_u64_le(data, &mut offset, "reverse patch length")? as usize;
        Some(read_var_bytes(
            data,
            &mut offset,
            reverse_len,
            "reverse patch data",
        )?)
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
