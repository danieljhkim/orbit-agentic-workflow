use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentToolCall {
    pub name: String,
    pub input: Value,
    pub output: Option<Value>,
    pub success: bool,
}
