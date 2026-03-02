use std::fs;
use std::path::Path;

use orbit_types::OrbitError;

#[cfg(unix)]
pub(crate) fn create_dir_symlink(src: &Path, dst: &Path) -> Result<(), OrbitError> {
    std::os::unix::fs::symlink(src, dst).map_err(|e| OrbitError::Io(e.to_string()))
}

#[cfg(windows)]
pub(crate) fn create_dir_symlink(src: &Path, dst: &Path) -> Result<(), OrbitError> {
    std::os::windows::fs::symlink_dir(src, dst).map_err(|e| OrbitError::Io(e.to_string()))
}

pub(crate) fn remove_path_if_exists(path: &Path) -> Result<(), OrbitError> {
    if !path.exists() {
        return Ok(());
    }

    let metadata = fs::symlink_metadata(path).map_err(|e| OrbitError::Io(e.to_string()))?;
    if metadata.file_type().is_symlink() {
        fs::remove_file(path).map_err(|e| OrbitError::Io(e.to_string()))
    } else if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|e| OrbitError::Io(e.to_string()))
    } else {
        fs::remove_file(path).map_err(|e| OrbitError::Io(e.to_string()))
    }
}

pub(crate) fn write_text_with_parent(path: &Path, content: &str) -> Result<(), OrbitError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;
    }
    fs::write(path, content).map_err(|e| OrbitError::Io(e.to_string()))
}
