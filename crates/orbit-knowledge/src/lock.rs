//! Shared file-based lock store for graph node editing.
//!
//! Locks live at `.orbit/knowledge/graph_locks.json` and are visible across all
//! tasks/agents.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::KnowledgeError;
use crate::io::write_text_atomic_durable;

/// A lock held on a graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeLock {
    pub owner: String,
    pub task_id: Option<String>,
    pub locked_at: String,
    pub reason: String,
}

/// Shared lock store backed by a JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LockStore {
    /// selector string → lock
    pub locks: HashMap<String, NodeLock>,
}

impl LockStore {
    /// Load the lock store from disk. Returns empty store if file doesn't exist.
    pub fn load(path: &Path) -> Result<Self, KnowledgeError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .map_err(|e| KnowledgeError::io(format!("read lock store: {e}")))?;
        serde_json::from_str(&raw)
            .map_err(|e| KnowledgeError::invalid_data(format!("parse lock store: {e}")))
    }

    /// Persist the lock store to disk atomically (write tmp + rename).
    pub fn save(&self, path: &Path) -> Result<(), KnowledgeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| KnowledgeError::io(format!("mkdir for lock store: {e}")))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| KnowledgeError::invalid_data(format!("serialize lock store: {e}")))?;
        write_text_atomic_durable(path, &format!("{json}\n"))
            .map_err(|e| KnowledgeError::io(format!("write lock store: {e}")))?;
        Ok(())
    }

    /// Acquire a lock. Fails if locked by a different owner.
    pub fn lock(
        &mut self,
        selector: &str,
        owner: &str,
        task_id: Option<&str>,
        reason: &str,
    ) -> Result<(), LockError> {
        if let Some(existing) = self.locks.get(selector) {
            if existing.owner != owner {
                return Err(LockError::Contention {
                    selector: selector.to_string(),
                    held_by: existing.owner.clone(),
                    locked_at: existing.locked_at.clone(),
                });
            }
            // Already locked by same owner — idempotent
            return Ok(());
        }
        self.locks.insert(
            selector.to_string(),
            NodeLock {
                owner: owner.to_string(),
                task_id: task_id.map(str::to_string),
                locked_at: Utc::now().to_rfc3339(),
                reason: reason.to_string(),
            },
        );
        Ok(())
    }

    /// Release a lock. Only the owner can unlock.
    pub fn unlock(&mut self, selector: &str, owner: &str) -> Result<(), LockError> {
        if let Some(existing) = self.locks.get(selector)
            && existing.owner != owner
        {
            return Err(LockError::Contention {
                selector: selector.to_string(),
                held_by: existing.owner.clone(),
                locked_at: existing.locked_at.clone(),
            });
        }
        self.locks.remove(selector);
        Ok(())
    }

    /// Check if a selector is locked by someone other than `owner`.
    pub fn check(&self, selector: &str, owner: &str) -> Result<(), LockError> {
        if let Some(existing) = self.locks.get(selector)
            && existing.owner != owner
        {
            return Err(LockError::Contention {
                selector: selector.to_string(),
                held_by: existing.owner.clone(),
                locked_at: existing.locked_at.clone(),
            });
        }
        Ok(())
    }

    /// Get the lock for a selector, if any.
    pub fn get(&self, selector: &str) -> Option<&NodeLock> {
        self.locks.get(selector)
    }
}

/// Held graph lock selectors that are released on drop, including unwind.
pub struct GraphLockGuard {
    lock_path: PathBuf,
    owner: String,
    selectors: Vec<String>,
    released: bool,
}

impl GraphLockGuard {
    pub fn acquire(
        knowledge_dir: &Path,
        owner: &str,
        task_id: Option<&str>,
        reason: &str,
        selectors: &[String],
    ) -> Result<Self, KnowledgeError> {
        let lock_path = lock_store_path(knowledge_dir);
        with_lock_store(&lock_path, |store| {
            for selector in selectors {
                store.lock(selector, owner, task_id, reason)?;
            }
            Ok(())
        })?;

        Ok(Self {
            lock_path,
            owner: owner.to_string(),
            selectors: selectors.to_vec(),
            released: false,
        })
    }

    pub fn release(&mut self) -> Result<(), KnowledgeError> {
        if self.released {
            return Ok(());
        }

        with_lock_store(&self.lock_path, |store| {
            for selector in &self.selectors {
                store.unlock(selector, &self.owner)?;
            }
            Ok(())
        })?;
        self.released = true;
        Ok(())
    }
}

impl Drop for GraphLockGuard {
    fn drop(&mut self) {
        if self.released {
            return;
        }

        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Err(error) = self.release() {
                eprintln!(
                    "warning: failed to release graph locks for `{}`: {error}",
                    self.owner
                );
            }
        }));
    }
}

/// Error returned when a lock operation fails due to contention.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum LockError {
    Contention {
        selector: String,
        held_by: String,
        locked_at: String,
    },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Contention {
                selector,
                held_by,
                locked_at,
            } => write!(f, "`{selector}` is locked by `{held_by}` since {locked_at}"),
        }
    }
}

/// Convenience: load → operate → save in one call.
pub fn with_lock_store<F, T>(lock_path: &Path, f: F) -> Result<T, KnowledgeError>
where
    F: FnOnce(&mut LockStore) -> Result<T, LockError>,
{
    let _sentinel = lock_store_sentinel(lock_path)?;
    let mut store = LockStore::load(lock_path)?;
    let result = f(&mut store).map_err(|e| KnowledgeError::invalid_data(e.to_string()))?;
    store.save(lock_path)?;
    Ok(result)
}

/// Default lock file path within the shared knowledge directory.
pub fn lock_store_path(knowledge_dir: &Path) -> PathBuf {
    knowledge_dir.join("graph_locks.json")
}

fn lock_store_sentinel(lock_path: &Path) -> Result<File, KnowledgeError> {
    let sentinel_path = lock_path.with_extension("json.lock");
    if let Some(parent) = sentinel_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| KnowledgeError::io(format!("mkdir for lock sentinel: {error}")))?;
    }

    let sentinel = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&sentinel_path)
        .map_err(|error| KnowledgeError::io(format!("open lock sentinel: {error}")))?;
    sentinel
        .lock_exclusive()
        .map_err(|error| KnowledgeError::io(format!("lock sentinel: {error}")))?;
    Ok(sentinel)
}
