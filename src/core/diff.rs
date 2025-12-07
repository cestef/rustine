use std::io::Write;

use crate::{Result, RustineErrorKind};

/// Generate binary diff/patch
pub fn create(base: &[u8], target: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();

    let bsdiff = qbsdiff::Bsdiff::new(base, target).parallel_scheme(qbsdiff::ParallelScheme::Auto);

    bsdiff
        .compare(&mut out)
        .map_err(|e| RustineErrorKind::DiffFailed { source: e })?;

    Ok(out)
}

/// Stream diff to writer (for large files)
pub fn write_to<W: Write>(base: &[u8], target: &[u8], writer: &mut W) -> Result<u64> {
    let bsdiff = qbsdiff::Bsdiff::new(base, target).parallel_scheme(qbsdiff::ParallelScheme::Auto);

    bsdiff
        .compare(writer)
        .map_err(|e| RustineErrorKind::DiffFailed { source: e }.into())
}
