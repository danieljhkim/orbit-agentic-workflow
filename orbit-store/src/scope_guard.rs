use std::path::{Path, PathBuf};

use orbit_types::OrbitError;

use crate::backend::ScopeResolution;

/// Enforces scoping rules on write operations.
///
/// Each file store holds a `ScopeGuard` that validates whether a write target
/// is permitted under the store's [`ScopeResolution`] strategy.
pub struct ScopeGuard {
    pub resolution: ScopeResolution,
    pub global_dir: PathBuf,
}

impl ScopeGuard {
    pub fn new(resolution: ScopeResolution, global_dir: PathBuf) -> Self {
        Self {
            resolution,
            global_dir,
        }
    }

    /// Returns `Ok(())` if the write is allowed under the current resolution,
    /// or `Err(ScopeViolation)` if it violates scoping rules.
    ///
    /// Enforcement is skipped when `global_dir` is empty (permissive guard)
    /// or when the target path and global_dir resolve to the same root
    /// (single-root mode where global == workspace).
    pub fn check_write(&self, target_path: &Path) -> Result<(), OrbitError> {
        if self.global_dir.as_os_str().is_empty() {
            return Ok(());
        }
        match self.resolution {
            ScopeResolution::WorkspaceOnly => {
                if target_path.starts_with(&self.global_dir) {
                    return Err(OrbitError::ScopeViolation(format!(
                        "WorkspaceOnly store cannot write to global path: {}",
                        target_path.display()
                    )));
                }
            }
            ScopeResolution::GlobalOnly => {
                if !target_path.starts_with(&self.global_dir) {
                    return Err(OrbitError::ScopeViolation(format!(
                        "GlobalOnly store cannot write outside global path: {}",
                        target_path.display()
                    )));
                }
            }
            // MergeByKey and WorkspaceReplaces: routing is handled by layered
            // stores, so writes are always allowed at the individual store level.
            ScopeResolution::MergeByKey | ScopeResolution::WorkspaceReplaces => {}
        }
        Ok(())
    }

    /// No-op guard that allows all writes. Used for backwards compatibility
    /// and in tests where scope enforcement is not under test.
    pub fn permissive() -> Self {
        Self {
            resolution: ScopeResolution::MergeByKey,
            global_dir: PathBuf::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn workspace_only_rejects_global_dir() {
        let guard = ScopeGuard::new(
            ScopeResolution::WorkspaceOnly,
            PathBuf::from("/home/user/.orbit"),
        );
        let result = guard.check_write(Path::new("/home/user/.orbit/tasks"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("WorkspaceOnly"));
    }

    #[test]
    fn workspace_only_allows_workspace_dir() {
        let guard = ScopeGuard::new(
            ScopeResolution::WorkspaceOnly,
            PathBuf::from("/home/user/.orbit"),
        );
        let result = guard.check_write(Path::new("/repo/.orbit/tasks"));
        assert!(result.is_ok());
    }

    #[test]
    fn global_only_rejects_workspace_dir() {
        let guard = ScopeGuard::new(
            ScopeResolution::GlobalOnly,
            PathBuf::from("/home/user/.orbit"),
        );
        let result = guard.check_write(Path::new("/repo/.orbit/tasks"));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("GlobalOnly"));
    }

    #[test]
    fn global_only_allows_global_dir() {
        let guard = ScopeGuard::new(
            ScopeResolution::GlobalOnly,
            PathBuf::from("/home/user/.orbit"),
        );
        let result = guard.check_write(Path::new("/home/user/.orbit/audit"));
        assert!(result.is_ok());
    }

    #[test]
    fn merge_by_key_allows_both() {
        let guard = ScopeGuard::new(
            ScopeResolution::MergeByKey,
            PathBuf::from("/home/user/.orbit"),
        );
        assert!(guard.check_write(Path::new("/home/user/.orbit/activities")).is_ok());
        assert!(guard.check_write(Path::new("/repo/.orbit/activities")).is_ok());
    }

    #[test]
    fn permissive_allows_everything() {
        let guard = ScopeGuard::permissive();
        assert!(guard.check_write(Path::new("/any/path")).is_ok());
        assert!(guard.check_write(Path::new("/home/user/.orbit/tasks")).is_ok());
    }
}
