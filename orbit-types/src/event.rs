use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum OrbitEvent {
    ToolExecuted {
        name: String,
    },
    ToolAdded {
        name: String,
    },
    ToolRemoved {
        name: String,
    },
    ToolEnabled {
        name: String,
    },
    ToolDisabled {
        name: String,
    },
    JobAdded {
        job_id: String,
    },
    JobPaused {
        job_id: String,
    },
    JobResumed {
        job_id: String,
    },
    JobDeleted {
        job_id: String,
    },
    JobTriggered {
        job_id: String,
    },
    JobRunStarted {
        job_id: String,
        run_id: String,
        attempt: u32,
    },
    JobRunCompleted {
        job_id: String,
        run_id: String,
        state: String,
    },
    JobRetryScheduled {
        job_id: String,
        run_id: String,
        next_run_at: String,
    },
    JobProtocolViolation {
        job_id: String,
        run_id: String,
        message: String,
    },
    JobSkipped {
        job_id: String,
        reason: String,
    },
    WatchTriggered {
        path: String,
    },
    PolicyDenied {
        tool: String,
    },
    TaskAdded {
        id: String,
    },
    TaskUpdated {
        id: String,
    },
    TaskClosed {
        id: String,
    },
    TaskReopened {
        id: String,
    },
    TaskDeleted {
        id: String,
    },
    SkillAdded {
        name: String,
    },
    SkillUpdated {
        name: String,
    },
    SkillDeleted {
        name: String,
    },
    SkillAttached {
        task_id: String,
        skill_name: String,
    },
    SkillDetached {
        task_id: String,
        skill_name: String,
    },
    ExecutionSpecAdded {
        id: String,
    },
    ExecutionSpecDisabled {
        id: String,
    },
    WorkflowAdded {
        id: String,
    },
    WorkflowDisabled {
        id: String,
    },
    AgentSessionStarted {
        session_id: String,
        task_id: String,
        skill_names: Vec<String>,
        composed_context_hash: String,
        effective_allowed_tools: Vec<String>,
    },
    AgentToolCall {
        session_id: String,
        task_id: String,
        skill_names: Vec<String>,
        tool_name: String,
        input: Value,
        output: Option<Value>,
        success: bool,
    },
    AgentSessionCompleted {
        session_id: String,
        task_id: String,
        status: String,
    },
    EntryCreated {
        id: String,
        entity_type: String,
        entity_id: String,
        sequence_number: i64,
    },
}
