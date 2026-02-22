pub mod audit;
pub mod error;
pub mod event;
pub mod id;
pub mod job;
pub mod memo;
pub mod task;
pub mod tool;
pub mod watch;

pub use audit::Audit;
pub use error::OrbitError;
pub use event::OrbitEvent;
pub use id::OrbitId;
pub use job::{Job, JobStatus};
pub use memo::Memo;
pub use task::{Task, TaskPriority, TaskStatus, TaskType};
pub use tool::{ExecutionResult, PolicyDecision, StoredTool, ToolParam, ToolSchema};
pub use watch::Watch;

#[cfg(test)]
mod tests {
    use crate::{ExecutionResult, OrbitEvent};

    #[test]
    fn orbit_event_shape_is_stable() {
        let event = OrbitEvent::ToolExecuted {
            name: "fs.read".to_string(),
        };
        let json = serde_json::to_value(event).expect("serialize event");

        assert_eq!(json["type"], "ToolExecuted");
        assert_eq!(json["data"]["name"], "fs.read");
    }

    #[test]
    fn execution_result_round_trips() {
        let result = ExecutionResult {
            success: true,
            stdout: "ok".to_string(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 12,
            output: Some(serde_json::json!({"k": "v"})),
        };

        let json = serde_json::to_string(&result).expect("serialize result");
        let decoded: ExecutionResult = serde_json::from_str(&json).expect("deserialize result");

        assert_eq!(decoded, result);
    }
}
