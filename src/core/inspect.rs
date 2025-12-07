use crate::{Result, RustineErrorKind};

/// Information about a patch file
#[derive(Debug)]
pub struct PatchInfo {
    pub patch_size: u64,
    pub expected_output_size: u64,
    pub format_version: String,
    pub is_valid: bool,
    pub has_checksums: bool,
    pub has_reverse: bool,
    pub base_checksum: Option<String>,
    pub output_checksum: Option<String>,
}

/// Inspect a patch file and extract metadata
pub fn inspect(patch_file_data: &[u8]) -> Result<PatchInfo> {
    // Deserialize using new format
    let patch = super::format::PatchData::deserialize(patch_file_data)?;

    // Try to parse the forward patch header to validate
    let is_valid = qbsdiff::Bspatch::new(&patch.forward_patch).is_ok();

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
    let patch_size = patch_file_data.len() as u64;
    let expected_output_size = if patch.forward_patch.len() >= 32 {
        i64::from_le_bytes(patch.forward_patch[24..32].try_into().unwrap()) as u64
    } else {
        0
    };

    // Determine format version
    let format_version = if patch_file_data.len() >= 8 && &patch_file_data[0..8] == b"RUSTINE2" {
        "RUSTINE2".to_string()
    } else if patch_file_data.len() >= 8 && &patch_file_data[0..8] == b"RUSTINE1" {
        "RUSTINE1".to_string()
    } else {
        "BSDIFF4".to_string()
    };

    Ok(PatchInfo {
        patch_size,
        expected_output_size,
        format_version,
        is_valid,
        has_checksums: patch.base_checksum.is_some() || patch.output_checksum.is_some(),
        has_reverse: patch.reverse_patch.is_some(),
        base_checksum: patch.base_checksum.map(|h| super::format::hex_encode_public(&h)),
        output_checksum: patch.output_checksum.map(|h| super::format::hex_encode_public(&h)),
    })
}
