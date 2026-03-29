//! PR scoreboard auto-increment.
//!
//! Updates `.orbit/scoreboard/pr.json` when PR lifecycle events occur:
//! - **merge**: increment `prs-merged`
//! - **revision**: increment `revisions` (each review loop iteration)
//! - **comment resolved**: increment `comments-resolved` (implementer fixed a flagged issue)

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use orbit_types::OrbitError;

type AgentScores = HashMap<String, HashMap<String, u64>>;
type Scoreboard = HashMap<String, AgentScores>;

/// Increment the `prs-merged` counter for the given agent/model.
pub fn record_pr_merged(scoreboard_dir: &Path, agent: &str, model: &str) -> Result<(), OrbitError> {
    increment(scoreboard_dir, "prs-merged", agent, model)
}

/// Increment the `revisions` counter for the given agent/model.
pub fn record_pr_revision(
    scoreboard_dir: &Path,
    agent: &str,
    model: &str,
) -> Result<(), OrbitError> {
    increment(scoreboard_dir, "revisions", agent, model)
}

/// Increment the `comments-resolved` counter for the given agent/model.
pub fn record_comment_resolved(
    scoreboard_dir: &Path,
    agent: &str,
    model: &str,
) -> Result<(), OrbitError> {
    increment(scoreboard_dir, "comments-resolved", agent, model)
}

fn increment(
    scoreboard_dir: &Path,
    metric: &str,
    agent: &str,
    model: &str,
) -> Result<(), OrbitError> {
    let path = scoreboard_dir.join("pr.json");
    let mut scoreboard: Scoreboard = if path.exists() {
        let content =
            fs::read_to_string(&path).map_err(|e| OrbitError::Io(format!("read pr.json: {e}")))?;
        serde_json::from_str(&content).map_err(|e| OrbitError::Io(format!("parse pr.json: {e}")))?
    } else {
        HashMap::new()
    };

    let agent_map = scoreboard.entry(metric.to_string()).or_default();
    let model_map = agent_map.entry(agent.to_string()).or_default();
    let counter = model_map.entry(model.to_string()).or_insert(0);
    *counter += 1;

    let json = serde_json::to_string_pretty(&scoreboard)
        .map_err(|e| OrbitError::Io(format!("serialize pr.json: {e}")))?;

    // Write atomically via temp file + rename
    let dir = path
        .parent()
        .ok_or_else(|| OrbitError::Io("no parent dir for pr.json".to_string()))?;
    fs::create_dir_all(dir).map_err(|e| OrbitError::Io(format!("create scoreboard dir: {e}")))?;

    let tmp = dir.join(".pr.json.tmp");
    fs::write(&tmp, format!("{json}\n"))
        .map_err(|e| OrbitError::Io(format!("write pr.json tmp: {e}")))?;
    fs::rename(&tmp, &path).map_err(|e| OrbitError::Io(format!("rename pr.json tmp: {e}")))?;

    Ok(())
}
