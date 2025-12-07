use std::io::Write;

use crate::{Result, RustineErrorKind};

/// Apply patch to base, return result
pub fn apply(base: &[u8], patch_data: &[u8]) -> Result<Vec<u8>> {
    let patcher = qbsdiff::Bspatch::new(patch_data)
        .map_err(|e| RustineErrorKind::InvalidPatch { source: e })?;

    let mut out = Vec::new();
    patcher
        .apply(base, &mut out)
        .map_err(|e| RustineErrorKind::PatchFailed { source: e })?;

    Ok(out)
}

/// Stream patch to writer
pub fn write_to<W: Write>(base: &[u8], patch_data: &[u8], writer: &mut W) -> Result<u64> {
    let patcher = qbsdiff::Bspatch::new(patch_data)
        .map_err(|e| RustineErrorKind::InvalidPatch { source: e })?;

    patcher
        .apply(base, writer)
        .map_err(|e| RustineErrorKind::PatchFailed { source: e }.into())
}
