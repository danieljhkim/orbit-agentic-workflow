use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum OrbitEvent {
    ToolExecuted { name: String },
    ToolAdded { name: String },
    ToolRemoved { name: String },
    ToolEnabled { name: String },
    ToolDisabled { name: String },
    JobStarted { id: String },
    JobCompleted { id: String, success: bool },
    WatchTriggered { path: String },
    PolicyDenied { tool: String },
    TaskAdded { id: String },
    TaskUpdated { id: String },
    TaskClosed { id: String },
    TaskReopened { id: String },
    TaskDeleted { id: String },
}
