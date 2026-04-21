//! `orbit task history` subcommand (T20260421-0528).
//!
//! Queries task-ID attribution for an `orbit.graph` selector. Prefers the
//! graph-backed path; falls back to `git log` + regex scan when the knowledge
//! graph or task-commits sidecar is unavailable.
//!
//! The `rebuild` subsubcommand is a thin wrapper that triggers
//! `orbit::pipeline::run_build` so operators have a single ergonomic entry
//! point for the attribute-history pass.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use orbit_knowledge::Selector;
use orbit_knowledge::graph::object_store::{RefName, resolve_graph_read_target};
use orbit_knowledge::pipeline::context::BuildConfig;
use orbit_knowledge::{DEFAULT_STALENESS_THRESHOLD, HistoryQueryOptions, query_task_history};
use serde_json::json;

use crate::command::Execute;

#[derive(Args)]
#[command(
    about = "Query task-ID history for a knowledge-graph selector",
    long_about = "Query task-ID history for a knowledge-graph selector.\n\n\
        Prefers the graph-backed path (task_ids on the node plus a task-commits \
        sidecar); falls back to a `git log` scan when either is missing. The \
        fallback emits a stderr warning and does not follow renames or moves.\n\n\
        Known limitation (T20260421-0528): the identity matcher recognises \
        symbols that share their file path and qualified name across commits; \
        it does not follow renames (different name), cross-file moves, or \
        body-only extractions. After a rename-heavy PR, run `orbit task \
        history rebuild` and accept that prior history on the renamed/moved \
        symbols is surfaced as fresh nodes with empty task_ids until a future \
        operation-log integration lands (follow-up T20260421-0543)."
)]
pub struct TaskHistoryCommand {
    #[command(subcommand)]
    pub subcommand: Option<TaskHistorySubcommand>,

    /// Selector to query (e.g. `file:src/lib.rs`,
    /// `symbol:src/lib.rs#hello:function`, `dir:src`). Required unless a
    /// subcommand is provided.
    #[arg(required_unless_present = "subcommand")]
    pub selector: Option<String>,

    /// Output as JSON.
    #[arg(long)]
    pub json: bool,

    /// Knowledge-graph ref name (defaults to the current git branch).
    #[arg(long = "ref")]
    pub ref_name: Option<String>,

    /// Staleness threshold (commits). A warning is emitted to stderr when
    /// HEAD is more than this many commits ahead of the attribution cursor.
    #[arg(long, default_value_t = DEFAULT_STALENESS_THRESHOLD)]
    pub staleness_threshold: u64,
}

#[derive(Subcommand)]
pub enum TaskHistorySubcommand {
    /// Rebuild the task-ID attribution for the current branch. Equivalent to
    /// `orbit graph build` — the attribution pass runs as part of every
    /// rebuild. Renamed or moved symbols after a rename-heavy PR are only
    /// picked up after this rebuild runs.
    Rebuild(TaskHistoryRebuildArgs),
}

#[derive(Args)]
pub struct TaskHistoryRebuildArgs {
    /// Repository root (defaults to current working directory).
    #[arg(long)]
    pub repo: Option<PathBuf>,

    /// Knowledge-graph ref name (defaults to the current git branch).
    #[arg(long = "ref")]
    pub ref_name: Option<String>,
}

impl Execute for TaskHistoryCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.subcommand {
            Some(TaskHistorySubcommand::Rebuild(args)) => args.execute(runtime),
            None => {
                let selector = self.selector.ok_or_else(|| {
                    OrbitError::InvalidInput(
                        "missing <selector>; run `orbit task history <selector>`".to_string(),
                    )
                })?;
                run_query(
                    runtime,
                    &selector,
                    self.json,
                    self.ref_name.as_deref(),
                    self.staleness_threshold,
                )
            }
        }
    }
}

impl Execute for TaskHistoryRebuildArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        run_rebuild(runtime, self.repo, self.ref_name)
    }
}

fn run_query(
    runtime: &OrbitRuntime,
    raw_selector: &str,
    as_json: bool,
    explicit_ref: Option<&str>,
    staleness_threshold: u64,
) -> Result<(), OrbitError> {
    let selector: Selector =
        raw_selector
            .parse()
            .map_err(|error: orbit_knowledge::SelectorParseError| {
                OrbitError::InvalidInput(error.to_string())
            })?;

    let data_root = runtime.data_root();
    let knowledge_dir = data_root.join("knowledge");
    let repo_path = data_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    let read_target = resolve_graph_read_target(Some(&repo_path), explicit_ref)?;
    let branch_ref = read_target.requested.clone();

    let options = HistoryQueryOptions {
        knowledge_dir: &knowledge_dir,
        repo_path: &repo_path,
        branch_ref: &branch_ref,
        selector: &selector,
        staleness_threshold,
    };

    let result = query_task_history(&options)
        .map_err(|error| OrbitError::Execution(format!("orbit task history failed: {error}")))?;

    for warning in &result.warnings {
        eprintln!("warning: {warning}");
    }

    if as_json {
        let payload = json!({
            "selector": result.selector,
            "source": result.source,
            "task_history": result.task_history,
            "staleness": result.staleness,
            "structural_conflict": result.structural_conflict,
        });
        crate::output::json::print_pretty(&payload)?;
    } else {
        print_human(&result);
    }

    Ok(())
}

fn run_rebuild(
    runtime: &OrbitRuntime,
    repo_override: Option<PathBuf>,
    ref_name: Option<String>,
) -> Result<(), OrbitError> {
    let data_root = runtime.data_root();
    let repo_path = repo_override.unwrap_or_else(|| {
        data_root
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let output_dir = data_root.join("knowledge");

    eprintln!(
        "task history rebuild: scanning {} (full history backfill)",
        repo_path.display()
    );

    let config = BuildConfig {
        repo_path,
        output_dir,
        incremental: false,
        ref_name: parse_ref_name(ref_name)?,
    };

    let ctx = orbit_knowledge::pipeline::run_build(config)
        .map_err(|error| OrbitError::Execution(format!("task history rebuild failed: {error}")))?;

    eprintln!(
        "task history rebuild: {} dirs, {} files, {} leaves",
        ctx.graph.dirs.len(),
        ctx.graph.files.len(),
        ctx.graph.leaves.len(),
    );
    eprintln!(
        "task history rebuild: written to {}",
        ctx.output_dir.display()
    );

    Ok(())
}

fn print_human(result: &orbit_knowledge::TaskHistoryResult) {
    use crate::output::color::{bold, dimmed};

    println!("{} {}", bold("Selector:"), result.selector);
    println!(
        "{} {}",
        bold("Source:"),
        format!("{:?}", result.source).to_lowercase()
    );
    if result.structural_conflict {
        println!("{}", bold("Structural conflict: true"));
    }
    if result.task_history.is_empty() {
        println!("{}", dimmed("No task IDs attributed to this node."));
        return;
    }
    for entry in &result.task_history {
        println!("{} {}", bold("Task:"), entry.task_id);
        if entry.commits.is_empty() {
            println!("  {}", dimmed("(no commits found in sidecar)"));
            continue;
        }
        for commit in &entry.commits {
            let short_sha: String = commit.sha.chars().take(12).collect();
            println!("  {} {} {}", short_sha, commit.date, commit.summary);
        }
    }
}

fn parse_ref_name(ref_name: Option<String>) -> Result<Option<RefName>, OrbitError> {
    ref_name
        .filter(|value| !value.trim().is_empty())
        .map(RefName::new)
        .transpose()
        .map_err(|error| OrbitError::InvalidInput(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ref_name_rejects_empty() {
        assert!(parse_ref_name(Some("".to_string())).unwrap().is_none());
    }

    #[test]
    fn parse_ref_name_accepts_valid() {
        let result = parse_ref_name(Some("agent-main".to_string())).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_str(), "agent-main");
    }

    #[test]
    fn parse_ref_name_none_is_none() {
        assert!(parse_ref_name(None).unwrap().is_none());
    }
}
