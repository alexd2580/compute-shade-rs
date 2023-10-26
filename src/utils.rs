use std::path::Path;

use filetime::FileTime;

use crate::error::{Error, VResult};

pub fn mtime(path: &Path) -> VResult<FileTime> {
    let metadata = path.metadata().map_err(|err| {
        Error::Local(format!("File '{}' cannot be read: {err:?}", path.display()))
    })?;
    Ok(FileTime::from_last_modification_time(&metadata))
}

/// alpha = 1 uses 100% of a. alpha = 0 uses 100% of b.
#[must_use]
pub fn mix(a: f32, b: f32, alpha: f32) -> f32 {
    a * alpha + b * (1f32 - alpha)
}
