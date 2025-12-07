use std::path::Path;

use crate::{Result, RustineErrorKind};

/// Validate file exists and is readable
pub fn exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(RustineErrorKind::FileNotFound {
            path: path.display().to_string(),
        }
        .into());
    }

    // Check if we can read it
    std::fs::metadata(path).map_err(|e| RustineErrorKind::FileUnreadable {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// Check if can overwrite, fail if file exists and !force
pub fn can_write(path: &Path, force: bool) -> Result<()> {
    if !force && path.exists() {
        return Err(RustineErrorKind::FileExists {
            path: path.display().to_string(),
        }
        .into());
    }
    Ok(())
}
