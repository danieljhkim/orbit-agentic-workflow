//! Task-history query service (T20260421-0528).
//!
//! Resolves `orbit task history <selector>` queries against the branch-aware
//! knowledge graph. Prefers the graph-backed path (`task_ids` on the node +
//! sidecar index); falls back to a `git log` + regex scan when either is
//! missing.
//!
//! The service emits a structurally stable result across both paths so callers
//! can rely on a single JSON shape regardless of whether the graph is
//! available. Warnings (fallback, staleness) are surfaced on the caller side
//! via the `warnings` field and must be routed to stderr by the CLI.

use std::path::{Path, PathBuf};

use regex::Regex;
use serde::Serialize;

use crate::error::KnowledgeError;
use crate::graph::object_store::{GraphObjectStore, RefName};
use crate::pipeline::history;
use crate::selector::Selector;
use crate::store::task_commits::{self, TaskCommitsIndex};
use crate::store::{KnowledgeStore, NodeTaskInfo};

const TASK_ID_REGEX_STR: &str = r"\[T\d{8}-\d{4}(?:-\d+)?\]";

/// Default threshold (in commits) past which the staleness warning fires.
/// Overridable via `HistoryQueryOptions::staleness_threshold`.
pub const DEFAULT_STALENESS_THRESHOLD: u64 = 50;

#[derive(Debug, Clone)]
pub struct HistoryQueryOptions<'a> {
    pub knowledge_dir: &'a Path,
    pub repo_path: &'a Path,
    pub branch_ref: &'a RefName,
    pub selector: &'a Selector,
    pub staleness_threshold: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskHistoryResult {
    pub selector: String,
    pub source: HistorySource,
    pub task_history: Vec<TaskHistoryEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staleness: Option<StalenessInfo>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub structural_conflict: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistorySource {
    Graph,
    GitLogFallback,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskHistoryEntry {
    pub task_id: String,
    pub commits: Vec<CommitSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitSummary {
    pub sha: String,
    pub date: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StalenessInfo {
    pub commits_behind: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    pub head: String,
    pub remediation: String,
}

pub fn query_task_history(
    options: &HistoryQueryOptions,
) -> Result<TaskHistoryResult, KnowledgeError> {
    if let Some(result) = try_graph_backed(options)? {
        return Ok(result);
    }
    fallback_git_log(options)
}

fn try_graph_backed(
    options: &HistoryQueryOptions,
) -> Result<Option<TaskHistoryResult>, KnowledgeError> {
    let knowledge_dir = options.knowledge_dir;
    if !knowledge_dir.is_dir() {
        return Ok(None);
    }

    let default_ref = git_default_ref(options.repo_path);
    let store = match KnowledgeStore::open(
        knowledge_dir,
        options.branch_ref,
        None,
        default_ref.as_ref(),
    ) {
        Ok(store) => store,
        Err(_) => return Ok(None),
    };

    let info = match store.node_task_info(options.selector)? {
        Some(info) => info,
        None => NodeTaskInfo {
            node_id: String::new(),
            node_type: String::new(),
            location: String::new(),
            task_ids: Vec::new(),
            structural_conflict: false,
        },
    };

    let sidecar_path = task_commits::sidecar_path(knowledge_dir, options.branch_ref.as_str());
    let sidecar = task_commits::load(&sidecar_path)?;

    if info.task_ids.is_empty() && sidecar.is_empty() && !sidecar_path.is_file() {
        // Graph is present but this branch has no sidecar yet (e.g. rebuild has
        // not yet run). Fall back so the user gets something.
        return Ok(None);
    }

    let mut warnings = Vec::new();
    let staleness = detect_staleness(options).unwrap_or_else(|error| {
        warnings.push(format!("staleness check skipped: {error}"));
        None
    });
    if let Some(staleness) = &staleness {
        warnings.push(format!(
            "knowledge graph is {} commits behind HEAD; run `{}`",
            staleness.commits_behind, staleness.remediation
        ));
    }

    let task_history = resolve_from_sidecar(&info.task_ids, &sidecar);

    Ok(Some(TaskHistoryResult {
        selector: options.selector.to_string(),
        source: HistorySource::Graph,
        task_history,
        staleness,
        structural_conflict: info.structural_conflict,
        warnings,
    }))
}

fn resolve_from_sidecar(task_ids: &[String], sidecar: &TaskCommitsIndex) -> Vec<TaskHistoryEntry> {
    task_ids
        .iter()
        .map(|task_id| {
            let commits = sidecar
                .get(task_id)
                .map(|entries| {
                    entries
                        .iter()
                        .map(|entry| CommitSummary {
                            sha: entry.sha.clone(),
                            date: entry.date.to_rfc3339(),
                            summary: entry.summary.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            TaskHistoryEntry {
                task_id: task_id.clone(),
                commits,
            }
        })
        .collect()
}

fn detect_staleness(
    options: &HistoryQueryOptions,
) -> Result<Option<StalenessInfo>, KnowledgeError> {
    let store = GraphObjectStore::new(options.knowledge_dir.join("graph"));
    let current_ref = match store.read_ref(options.branch_ref) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(cursor) = current_ref.last_attributed_commit else {
        return Ok(None);
    };
    let Some(head) = history::resolve_head(options.repo_path)? else {
        return Ok(None);
    };
    if cursor == head {
        return Ok(None);
    }
    let gap = history::count_commits(options.repo_path, &cursor, &head)?;
    if gap <= options.staleness_threshold {
        return Ok(None);
    }
    Ok(Some(StalenessInfo {
        commits_behind: gap,
        cursor: Some(cursor),
        head,
        remediation: "orbit task history rebuild".to_string(),
    }))
}

fn fallback_git_log(options: &HistoryQueryOptions) -> Result<TaskHistoryResult, KnowledgeError> {
    let mut warnings = Vec::new();
    warnings.push(
        "knowledge graph unavailable or task-commits sidecar missing — \
         falling back to `git log`; rename/move history is not available"
            .to_string(),
    );

    let selector_paths = selector_paths(options.selector);
    let mut args: Vec<String> = vec![
        "log".into(),
        "--reverse".into(),
        "--topo-order".into(),
        "--format=%H%x00%cI%x00%s%x00%B%x1e".into(),
    ];
    if !selector_paths.is_empty() {
        args.push("--".into());
        for path in &selector_paths {
            args.push(path.to_string_lossy().into_owned());
        }
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let output = orbit_common::utility::git::run_git(options.repo_path, &arg_refs)
        .map_err(|error| KnowledgeError::io(format!("git log failed: {error}")))?;
    let mut task_history: std::collections::BTreeMap<String, Vec<CommitSummary>> =
        std::collections::BTreeMap::new();
    if !output.success {
        warnings.push(format!(
            "git log returned non-zero exit status: {}",
            output.stderr.trim()
        ));
    } else {
        let regex = Regex::new(TASK_ID_REGEX_STR).expect("task-ID regex compiles");
        for record in output.stdout.split('\x1e') {
            let record = record.trim_matches('\n');
            if record.is_empty() {
                continue;
            }
            let parts: Vec<&str> = record.splitn(4, '\x00').collect();
            if parts.len() < 4 {
                continue;
            }
            let summary = CommitSummary {
                sha: parts[0].to_string(),
                date: parts[1].to_string(),
                summary: parts[2].to_string(),
            };
            for m in regex.find_iter(parts[3]) {
                let raw = m.as_str();
                let task_id = raw[1..raw.len() - 1].to_string();
                let entry = task_history.entry(task_id).or_default();
                if !entry.iter().any(|existing| existing.sha == summary.sha) {
                    entry.push(summary.clone());
                }
            }
        }
    }

    let history: Vec<TaskHistoryEntry> = task_history
        .into_iter()
        .map(|(task_id, commits)| TaskHistoryEntry { task_id, commits })
        .collect();

    Ok(TaskHistoryResult {
        selector: options.selector.to_string(),
        source: HistorySource::GitLogFallback,
        task_history: history,
        staleness: None,
        structural_conflict: false,
        warnings,
    })
}

fn selector_paths(selector: &Selector) -> Vec<PathBuf> {
    match selector {
        Selector::Dir { path } => vec![PathBuf::from(path.trim_end_matches('/'))],
        Selector::File { path } => vec![PathBuf::from(path)],
        Selector::Symbol { path, .. } => vec![PathBuf::from(path)],
    }
}

fn git_default_ref(repo_path: &Path) -> Option<RefName> {
    orbit_common::utility::git::default_branch(repo_path)
        .ok()
        .flatten()
        .and_then(|branch| RefName::new(branch).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};
    use tempfile::tempdir;

    use crate::graph::object_store::RefName;
    use crate::selector::Selector;

    fn write_empty_manifest(dir: &Path) {
        std::fs::write(
            dir.join("manifest.json"),
            r#"{"generated_at":"2026-04-21T00:00:00Z"}"#,
        )
        .unwrap();
    }

    fn make_options<'a>(
        knowledge_dir: &'a Path,
        repo_path: &'a Path,
        branch_ref: &'a RefName,
        selector: &'a Selector,
    ) -> HistoryQueryOptions<'a> {
        HistoryQueryOptions {
            knowledge_dir,
            repo_path,
            branch_ref,
            selector,
            staleness_threshold: DEFAULT_STALENESS_THRESHOLD,
        }
    }

    #[test]
    fn selector_paths_for_dir_trims_trailing_slash() {
        let s = Selector::Dir {
            path: "src/foo/".to_string(),
        };
        assert_eq!(selector_paths(&s), vec![PathBuf::from("src/foo")]);
    }

    #[test]
    fn selector_paths_for_file() {
        let s = Selector::File {
            path: "src/foo.rs".to_string(),
        };
        assert_eq!(selector_paths(&s), vec![PathBuf::from("src/foo.rs")]);
    }

    #[test]
    fn selector_paths_for_symbol_uses_file_path() {
        let s = Selector::Symbol {
            path: "src/foo.rs".to_string(),
            symbol: "bar".to_string(),
            kind: "function".to_string(),
        };
        assert_eq!(selector_paths(&s), vec![PathBuf::from("src/foo.rs")]);
    }

    #[test]
    fn resolve_from_sidecar_returns_empty_commits_for_unknown_task_id() {
        let sidecar = TaskCommitsIndex::default();
        let task_ids = vec!["T20260421-0528".to_string()];
        let history = resolve_from_sidecar(&task_ids, &sidecar);
        assert_eq!(history.len(), 1);
        assert!(history[0].commits.is_empty());
    }

    #[test]
    fn resolve_from_sidecar_maps_task_ids_to_commits() {
        let mut sidecar = TaskCommitsIndex::default();
        let date: DateTime<Utc> = DateTime::parse_from_rfc3339("2026-04-21T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        sidecar.append(
            "T20260421-0528".to_string(),
            crate::store::task_commits::CommitSummary {
                sha: "abc".to_string(),
                date,
                summary: "add task_ids".to_string(),
            },
        );
        let task_ids = vec!["T20260421-0528".to_string()];
        let history = resolve_from_sidecar(&task_ids, &sidecar);
        assert_eq!(history[0].commits.len(), 1);
        assert_eq!(history[0].commits[0].sha, "abc");
    }

    #[test]
    fn try_graph_backed_returns_none_when_knowledge_dir_missing() {
        let dir = tempdir().unwrap();
        let knowledge_dir = dir.path().join("nope");
        let repo = tempdir().unwrap();
        let branch = RefName::new("main").unwrap();
        let selector = Selector::File {
            path: "src/foo.rs".to_string(),
        };
        let options = make_options(&knowledge_dir, repo.path(), &branch, &selector);
        let result = try_graph_backed(&options).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn try_graph_backed_returns_none_when_store_open_fails() {
        let dir = tempdir().unwrap();
        let knowledge_dir = dir.path().join("knowledge");
        std::fs::create_dir_all(&knowledge_dir).unwrap();
        write_empty_manifest(&knowledge_dir);
        let repo = tempdir().unwrap();
        let branch = RefName::new("does-not-exist").unwrap();
        let selector = Selector::File {
            path: "src/foo.rs".to_string(),
        };
        let options = make_options(&knowledge_dir, repo.path(), &branch, &selector);
        let result = try_graph_backed(&options).unwrap();
        assert!(result.is_none());
    }
}
