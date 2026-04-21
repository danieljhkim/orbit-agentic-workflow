//! Sidecar index mapping `task_id -> [{sha, date, summary}]` for fast
//! history queries (T20260421-0528).
//!
//! Persisted as JSON alongside the per-branch graph ref at
//! `.orbit/knowledge/graph/refs/heads/<branch>.task-commits.json`. Written
//! atomically by the attribution pipeline stage; read by the
//! `orbit task history` query command.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::KnowledgeError;
use crate::io::write_text_atomic_durable;

/// Per-task-ID set of commit summaries. Keys are sorted (BTreeMap) so the
/// on-disk output is stable across rebuilds.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskCommitsIndex {
    #[serde(default)]
    pub entries: BTreeMap<String, Vec<CommitSummary>>,
}

/// Commit metadata the query command emits alongside each task ID.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommitSummary {
    pub sha: String,
    pub date: DateTime<Utc>,
    pub summary: String,
}

impl TaskCommitsIndex {
    /// Add a commit to the set for the given task ID. Duplicate SHAs are ignored.
    pub fn append(&mut self, task_id: String, summary: CommitSummary) {
        let entry = self.entries.entry(task_id).or_default();
        if !entry.iter().any(|existing| existing.sha == summary.sha) {
            entry.push(summary);
        }
    }

    /// Sort every task's commit list by date (then sha) and deduplicate by sha.
    /// Must be called before persisting for deterministic output.
    pub fn finalize(&mut self) {
        for list in self.entries.values_mut() {
            list.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.sha.cmp(&b.sha)));
            list.dedup_by(|a, b| a.sha == b.sha);
        }
    }

    pub fn get(&self, task_id: &str) -> Option<&[CommitSummary]> {
        self.entries.get(task_id).map(|v| v.as_slice())
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Path where the sidecar for a given branch lives.
pub fn sidecar_path(knowledge_dir: &Path, ref_name: &str) -> PathBuf {
    knowledge_dir
        .join("graph")
        .join("refs")
        .join("heads")
        .join(format!("{ref_name}.task-commits.json"))
}

/// Load the sidecar for a branch. Returns an empty index when the file does
/// not exist.
pub fn load(path: &Path) -> Result<TaskCommitsIndex, KnowledgeError> {
    if !path.is_file() {
        return Ok(TaskCommitsIndex::default());
    }
    let raw = std::fs::read_to_string(path)
        .map_err(|error| KnowledgeError::io(format!("read task-commits sidecar: {error}")))?;
    serde_json::from_str(&raw).map_err(|error| {
        KnowledgeError::invalid_data(format!("parse task-commits sidecar: {error}"))
    })
}

/// Persist the sidecar atomically (tempfile + rename + fsync parent).
pub fn save(path: &Path, index: &TaskCommitsIndex) -> Result<(), KnowledgeError> {
    let json = serde_json::to_string_pretty(index).map_err(|error| {
        KnowledgeError::invalid_data(format!("serialize task-commits sidecar: {error}"))
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| KnowledgeError::io(format!("create sidecar dir: {error}")))?;
    }
    write_text_atomic_durable(path, &format!("{json}\n"))
        .map_err(|error| KnowledgeError::io(format!("write task-commits sidecar: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sum(sha: &str, date: &str, summary: &str) -> CommitSummary {
        CommitSummary {
            sha: sha.to_string(),
            date: DateTime::parse_from_rfc3339(date)
                .unwrap()
                .with_timezone(&Utc),
            summary: summary.to_string(),
        }
    }

    #[test]
    fn append_dedupes_by_sha() {
        let mut index = TaskCommitsIndex::default();
        index.append(
            "T20260421-0528".to_string(),
            sum("abc123", "2026-04-21T00:00:00Z", "first"),
        );
        index.append(
            "T20260421-0528".to_string(),
            sum("abc123", "2026-04-21T00:00:00Z", "first (dup)"),
        );
        assert_eq!(index.get("T20260421-0528").unwrap().len(), 1);
    }

    #[test]
    fn finalize_sorts_by_date_then_sha() {
        let mut index = TaskCommitsIndex::default();
        index.append("T".to_string(), sum("aaa", "2026-04-21T02:00:00Z", "later"));
        index.append(
            "T".to_string(),
            sum("bbb", "2026-04-21T01:00:00Z", "earlier"),
        );
        index.append(
            "T".to_string(),
            sum("ccc", "2026-04-21T01:00:00Z", "earlier tie"),
        );
        index.finalize();
        let list = index.get("T").unwrap();
        assert_eq!(list[0].sha, "bbb");
        assert_eq!(list[1].sha, "ccc");
        assert_eq!(list[2].sha, "aaa");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sidecar.json");

        let mut index = TaskCommitsIndex::default();
        index.append(
            "T20260421-0528".to_string(),
            sum("abc", "2026-04-21T00:00:00Z", "add task_ids"),
        );
        index.append(
            "T20260421-0358".to_string(),
            sum("def", "2026-04-20T00:00:00Z", "branch refs"),
        );
        index.finalize();
        save(&path, &index).unwrap();

        let loaded = load(&path).unwrap();
        assert_eq!(loaded, index);
    }

    #[test]
    fn load_missing_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        let loaded = load(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn save_is_idempotent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sidecar.json");
        let mut index = TaskCommitsIndex::default();
        index.append(
            "T20260421-0528".to_string(),
            sum("abc", "2026-04-21T00:00:00Z", "x"),
        );
        index.finalize();
        save(&path, &index).unwrap();
        let first = std::fs::read_to_string(&path).unwrap();
        save(&path, &index).unwrap();
        let second = std::fs::read_to_string(&path).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn sidecar_path_joins_branch_name() {
        let p = sidecar_path(std::path::Path::new("/k"), "agent-main");
        assert_eq!(
            p,
            PathBuf::from("/k/graph/refs/heads/agent-main.task-commits.json")
        );
    }
}
