use std::fs;
use std::path::Path;

use serde_json::json;

use orbit_types::OrbitError;

use crate::Store;

pub fn write_token_scoreboard(scoreboard_dir: &Path, store: &Store) -> Result<(), OrbitError> {
    let path = scoreboard_dir.join("tokens.json");
    let payload = json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "activities": store.list_activity_invocation_metrics()?,
        "agents": store.list_agent_invocation_metrics()?,
        "top_tasks": store.list_top_task_invocation_metrics(20)?,
        "tools": store.list_tool_invocation_metrics()?,
        "known_limitations": [
            "Subagent attribution folds into the parent invocation totals.",
            "cache_read_tokens are reported separately from input_tokens.",
            "Multi-task invocations are fully attributed to every tagged task.",
            "Non-Claude providers currently emit zero traces."
        ]
    });

    fs::create_dir_all(scoreboard_dir).map_err(|e| OrbitError::Io(e.to_string()))?;
    let tmp = scoreboard_dir.join(".tokens.json.tmp");
    let raw = serde_json::to_string_pretty(&payload)
        .map_err(|e| OrbitError::Store(format!("serialize tokens.json: {e}")))?;
    fs::write(&tmp, format!("{raw}\n")).map_err(|e| OrbitError::Io(e.to_string()))?;
    fs::rename(&tmp, &path).map_err(|e| OrbitError::Io(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use orbit_types::{InvocationTrace, TokenUsage, ToolCallTrace};
    use serde_json::Value;
    use tempfile::tempdir;

    use crate::{InvocationInsertParams, Store};

    use super::write_token_scoreboard;

    fn seed_invocation(
        store: &Store,
        activity_id: &str,
        agent: &str,
        model: Option<&str>,
        input: u64,
        output: u64,
    ) {
        store
            .insert_invocation_trace_record(&InvocationInsertParams {
                job_run_id: format!("run-{activity_id}-{agent}"),
                activity_id: activity_id.to_string(),
                agent: agent.to_string(),
                model: model.map(|value| value.to_string()),
                task_ids: vec![],
                trace: InvocationTrace {
                    usage: TokenUsage {
                        input,
                        cache_read: 0,
                        cache_create: 0,
                        output,
                    },
                    tool_calls: vec![ToolCallTrace {
                        seq: 1,
                        tool_name: "fs.read".to_string(),
                        result_bytes: 8,
                        result_payload: None,
                    }],
                    duration_ms: 42,
                },
            })
            .expect("seed invocation");
    }

    fn find_row<'a>(
        rows: &'a [Value],
        activity_id: &str,
        agent: &str,
        model: Option<&str>,
    ) -> &'a Value {
        rows.iter()
            .find(|row| {
                row["activity_id"] == activity_id
                    && row["agent"] == agent
                    && row["model"] == model.map_or(Value::Null, Value::from)
            })
            .expect("row")
    }

    fn find_agent_row<'a>(rows: &'a [Value], agent: &str, model: Option<&str>) -> &'a Value {
        rows.iter()
            .find(|row| {
                row["agent"] == agent && row["model"] == model.map_or(Value::Null, Value::from)
            })
            .expect("row")
    }

    #[test]
    fn writes_agent_rollup_and_activity_identity_columns() {
        let dir = tempdir().expect("tempdir");
        let store = Store::open_in_memory().expect("store");

        seed_invocation(&store, "activity-a", "claude", Some("opus"), 10, 3);
        seed_invocation(&store, "activity-a", "codex", Some("gpt-5.4"), 7, 2);
        seed_invocation(&store, "activity-b", "claude", Some("opus"), 5, 1);

        write_token_scoreboard(dir.path(), &store).expect("write scoreboard");

        let payload: Value = serde_json::from_str(
            &fs::read_to_string(dir.path().join("tokens.json")).expect("tokens.json"),
        )
        .expect("json");

        let activities = payload["activities"].as_array().expect("activities");
        assert_eq!(activities.len(), 3);
        let activity_row = find_row(activities, "activity-a", "claude", Some("opus"));
        assert_eq!(activity_row["invocation_count"], 1);
        assert_eq!(activity_row["total_tokens"], 13);
        let agent_row = find_row(activities, "activity-a", "codex", Some("gpt-5.4"));
        assert_eq!(agent_row["invocation_count"], 1);
        assert_eq!(agent_row["total_tokens"], 9);

        let agents = payload["agents"].as_array().expect("agents");
        assert_eq!(agents.len(), 2);
        let claude_row = find_agent_row(agents, "claude", Some("opus"));
        assert_eq!(claude_row["invocation_count"], 2);
        assert_eq!(claude_row["total_tokens"], 19);
        let codex_row = find_agent_row(agents, "codex", Some("gpt-5.4"));
        assert_eq!(codex_row["invocation_count"], 1);
        assert_eq!(codex_row["total_tokens"], 9);
    }
}
