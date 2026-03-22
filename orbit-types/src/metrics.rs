use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A single metrics record captured at step completion.
///
/// Follows the same JSONL day-partitioned pattern as [`super::FrictionEntry`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsEntry {
    pub ts: DateTime<Utc>,
    pub job_run: String,
    pub step: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// Number of tool invocations executed during this step.
    #[serde(default)]
    pub tool_invocations: u32,
    /// Total token usage (input + output) for this step, if available.
    #[serde(default)]
    pub token_usage: Option<u64>,
    /// Wall-clock duration of this step in milliseconds.
    #[serde(default)]
    pub step_duration_ms: Option<u64>,
    /// Number of retries that occurred before step completion.
    #[serde(default)]
    pub retry_count: u32,
}
