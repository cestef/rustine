use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::{Result, RustineError, RustineErrorContext, RustineErrorKind, ui::Ctx};

use super::check;

// Threshold for streaming mode (100MB)
const STREAMING_THRESHOLD: u64 = 100 * 1024 * 1024;

/// Read file with UI feedback
pub fn read(path: &Path, ctx: &Ctx) -> Result<Vec<u8>> {
    ctx.msg(&format!(
        "Reading {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));

    std::fs::read(path).map_err(|e| {
        RustineError::new(
            RustineErrorKind::from(e),
            RustineErrorContext::default().with_path(path.to_path_buf()),
        )
    })
}

/// Read file with streaming for large files
pub fn read_streaming(path: &Path, ctx: &Ctx) -> Result<Vec<u8>> {
    let metadata = std::fs::metadata(path)?;
    let size = metadata.len();

    if size > STREAMING_THRESHOLD {
        ctx.msg(&format!(
            "Reading {} (streaming mode)",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));

        let file = File::open(path).map_err(|e| {
            RustineError::new(
                RustineErrorKind::from(e),
                RustineErrorContext::default().with_path(path.to_path_buf()),
            )
        })?;

        let mut reader = BufReader::new(file);
        let mut buffer = Vec::with_capacity(size as usize);
        reader.read_to_end(&mut buffer).map_err(|e| {
            RustineError::new(
                RustineErrorKind::from(e),
                RustineErrorContext::default().with_path(path.to_path_buf()),
            )
        })?;

        Ok(buffer)
    } else {
        read(path, ctx)
    }
}

/// Check if file should use streaming based on size
pub fn should_stream(path: &Path) -> Result<bool> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len() > STREAMING_THRESHOLD)
}

/// Write file with UI feedback and overwrite check
pub fn write(path: &Path, data: &[u8], force: bool, ctx: &Ctx) -> Result<u64> {
    check::can_write(path, force)?;
    ctx.msg(&format!(
        "Writing {}",
        path.file_name().unwrap_or_default().to_string_lossy()
    ));
    std::fs::write(path, data)?;
    Ok(data.len() as u64)
}
