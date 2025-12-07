use crate::{Result, RustineErrorKind};

/// Information about a patch file
#[derive(Debug)]
pub struct PatchInfo {
    pub patch_size: u64,
    pub expected_output_size: u64,
    pub format_version: String,
    pub is_valid: bool,
    pub has_checksums: bool,
    pub base_checksum: Option<String>,
    pub output_checksum: Option<String>,
}

/// Inspect a patch file and extract metadata
pub fn inspect(patch_data: &[u8]) -> Result<PatchInfo> {
    // Check for checksum wrapper
    let (base_hash, output_hash, actual_patch) = super::checksum::unwrap_patch(patch_data)?;

    // Try to parse the patch header
    let is_valid = qbsdiff::Bspatch::new(actual_patch).is_ok();

    if !is_valid {
        return Err(RustineErrorKind::InvalidPatch {
            source: std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid patch format",
            ),
        }
        .into());
    }

    // Parse bsdiff4 header manually to extract metadata
    // Header format: "BSDIFF40" (8 bytes) + ctrl_len (8) + diff_len (8) + new_size (8)
    let patch_size = patch_data.len() as u64;
    let expected_output_size = if actual_patch.len() >= 32 {
        i64::from_le_bytes(actual_patch[24..32].try_into().unwrap()) as u64
    } else {
        0
    };

    Ok(PatchInfo {
        patch_size,
        expected_output_size,
        format_version: if base_hash.is_some() {
            "RUSTINE1".to_string()
        } else {
            "BSDIFF4".to_string()
        },
        is_valid,
        has_checksums: base_hash.is_some(),
        base_checksum: base_hash.map(|h| super::checksum::hex_encode_public(&h)),
        output_checksum: output_hash.map(|h| super::checksum::hex_encode_public(&h)),
    })
}
