//! Utility-layer error type.
//!
//! `UtilError` is intentionally small. Utility helpers (git, fs, etc.) cannot
//! depend on the domain error in `orbit-types` because that would invert the
//! `orbit-util ← orbit-types` dependency direction. Instead, helpers return
//! `UtilError`, and `orbit-types::OrbitError` provides `From<UtilError>` so
//! callers' `?` ergonomics survive.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum UtilError {
    #[error("io error: {0}")]
    Io(String),
    #[error("execution failed: {0}")]
    Execution(String),
}

impl From<std::io::Error> for UtilError {
    fn from(err: std::io::Error) -> Self {
        UtilError::Io(err.to_string())
    }
}
