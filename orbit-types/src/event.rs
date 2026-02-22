use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum OrbitEvent {
    ToolExecuted { name: String },
    JobStarted { id: String },
    JobCompleted { id: String, success: bool },
    WatchTriggered { path: String },
    PolicyDenied { tool: String },
    TaskAdded { id: String },
}
