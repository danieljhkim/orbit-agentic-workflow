use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use chrono::Utc;
use fs2::FileExt;
use orbit_types::OrbitError;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Atomically writes `content` to `path` by writing to a timestamped `.tmp` sibling,
/// then renaming. Cleans up the temp file on rename failure.
pub(crate) fn write_atomic(path: &Path, content: &str) -> Result<(), OrbitError> {
    let parent = path.parent().ok_or_else(|| {
        OrbitError::Io(format!("cannot determine parent for '{}'", path.display()))
    })?;
    fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;

    let tmp_path = temp_path_for(path)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .truncate(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|e| OrbitError::Io(e.to_string()))?;
    file.write_all(content.as_bytes())
        .map_err(|e| OrbitError::Io(e.to_string()))?;
    drop(file);

    if let Err(err) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(OrbitError::Io(err.to_string()));
    }
    Ok(())
}

pub(crate) fn with_exclusive_file_lock<T, F>(
    target_path: &Path,
    label: &str,
    op: F,
) -> Result<T, OrbitError>
where
    F: FnOnce() -> Result<T, OrbitError>,
{
    let parent = target_path.parent().ok_or_else(|| {
        OrbitError::Io(format!(
            "cannot determine parent for '{}'",
            target_path.display()
        ))
    })?;
    fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;

    let lock_path = lock_path_for(target_path)?;
    let lock_file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| OrbitError::Io(format!("open {label} lock '{}': {e}", lock_path.display())))?;
    lock_file
        .lock_exclusive()
        .map_err(|e| OrbitError::Io(format!("lock {label} '{}': {e}", lock_path.display())))?;

    op()
}

fn temp_path_for(path: &Path) -> Result<PathBuf, OrbitError> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| OrbitError::Io(format!("path '{}' has no file name", path.display())))?;
    let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    Ok(path.with_file_name(format!(".{file_name}.tmp.{nanos}.{counter}")))
}

fn lock_path_for(path: &Path) -> Result<PathBuf, OrbitError> {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| OrbitError::Io(format!("path '{}' has no file name", path.display())))?;
    Ok(path.with_file_name(format!(".{file_name}.lock")))
}
