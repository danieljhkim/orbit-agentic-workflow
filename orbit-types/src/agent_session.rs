use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{IdentityRole, OrbitId};

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
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub identity_name: Option<String>,
    #[serde(default)]
    pub identity_role: Option<IdentityRole>,
    #[serde(default)]
    pub identity_block: Option<String>,
    pub skill_names: Vec<String>,
    pub composed_context_hash: String,
    pub effective_allowed_tools: Vec<String>,
    pub tool_calls: Vec<AgentToolCall>,
    pub outcome: String,
    pub status: AgentSessionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
