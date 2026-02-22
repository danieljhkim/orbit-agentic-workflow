use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{OrbitId, Role};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skill {
    pub schema_version: u8,
    pub name: String,
    pub description: Option<String>,
    pub instructions: String,
    pub context_files: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub role: Role,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSkillAttachment {
    pub task_id: OrbitId,
    pub skill_name: String,
    pub attachment_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentToolCall {
    pub name: String,
    pub input: Value,
    pub output: Option<Value>,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentSessionStatus {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentSession {
    pub session_id: OrbitId,
    pub task_id: OrbitId,
    pub skill_names: Vec<String>,
    pub composed_context_hash: String,
    pub effective_allowed_tools: Vec<String>,
    pub tool_calls: Vec<AgentToolCall>,
    pub outcome: String,
    pub status: AgentSessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
