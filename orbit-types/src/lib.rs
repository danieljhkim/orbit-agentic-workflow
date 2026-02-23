pub mod audit;
pub mod audit_event;
pub mod entry;
pub mod error;
pub mod event;
pub mod execution_spec;
pub mod id;
pub mod job;
pub mod memo;
pub mod role;
pub mod skill;
pub mod task;
pub mod tool;
pub mod watch;
pub mod workflow;

pub use audit::Audit;
pub use audit_event::{AuditEvent, AuditEventStatus, AuditStats};
pub use entry::{AuthorType, EntityType, Entry, EntryType};
pub use error::OrbitError;
pub use event::OrbitEvent;
pub use execution_spec::ExecutionSpec;
pub use id::OrbitId;
pub use job::{
    AgentResponseEnvelope, AgentRunError, Job, JobRetryBackoffStrategy, JobRun, JobRunState,
    JobScheduleState, JobSession, JobSessionStatus, JobTargetType, JobTrigger,
};
pub use memo::Memo;
pub use role::Role;
pub use skill::{AgentSession, AgentSessionStatus, AgentToolCall, Skill, TaskSkillAttachment};
pub use task::{Task, TaskPriority, TaskStatus, TaskType};
pub use tool::{ExecutionResult, PolicyDecision, StoredTool, ToolParam, ToolSchema};
pub use watch::Watch;
pub use workflow::Workflow;

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use crate::{
        AgentResponseEnvelope, ExecutionResult, ExecutionSpec, Job, JobRetryBackoffStrategy,
        JobRun, JobRunState, JobScheduleState, JobTargetType, OrbitEvent, Role, Skill, Workflow,
    };

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

    #[test]
    fn role_round_trips() {
        let role = Role::Agent;
        let json = serde_json::to_string(&role).expect("serialize role");
        assert_eq!(json, "\"agent\"");
        let decoded: Role = serde_json::from_str(&json).expect("deserialize role");
        assert_eq!(decoded, Role::Agent);
    }

    #[test]
    fn skill_shape_is_stable() {
        let skill = Skill {
            schema_version: 1,
            name: "refactor-rust-module".to_string(),
            description: Some("test".to_string()),
            instructions: "Do things".to_string(),
            context_files: vec!["ARCHITECTURE.md".to_string()],
            allowed_tools: vec!["fs.read".to_string()],
            role: Role::Agent,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let value = serde_json::to_value(skill).expect("serialize skill");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["name"], "refactor-rust-module");
        assert_eq!(value["role"], "agent");
    }

    #[test]
    fn entry_shape_is_stable() {
        let value = serde_json::json!({
            "id": "entry-1",
            "entity_type": "task",
            "entity_id": "task-1",
            "session_id": null,
            "sequence_number": 1,
            "entry_type": "comment",
            "author_type": "human",
            "author_id": "daniel",
            "author_model": null,
            "body": "hello",
            "created_at": "2026-02-22T00:00:00Z"
        });

        let decoded: crate::Entry = serde_json::from_value(value.clone()).expect("decode entry");
        let reencoded = serde_json::to_value(decoded).expect("encode entry");
        assert_eq!(reencoded["entity_type"], "task");
        assert_eq!(reencoded["entry_type"], "comment");
        assert_eq!(reencoded["author_type"], "human");
    }

    #[test]
    fn job_shapes_are_stable() {
        let job = Job {
            job_id: "job-1".to_string(),
            target_type: JobTargetType::ExecutionSpec,
            target_id: "exec-1".to_string(),
            schedule: "0 * * * *".to_string(),
            agent_cli: "claude".to_string(),
            timeout_seconds: 300,
            retry_max_attempts: 2,
            retry_backoff_strategy: JobRetryBackoffStrategy::Exponential,
            retry_initial_delay_seconds: 10,
            state: JobScheduleState::Enabled,
            next_run_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let job_value = serde_json::to_value(job).expect("serialize job");
        assert_eq!(job_value["state"], "enabled");
        assert_eq!(job_value["target_type"], "execution_spec");

        let run = JobRun {
            run_id: "run-1".to_string(),
            job_id: "job-1".to_string(),
            attempt: 1,
            state: JobRunState::Running,
            scheduled_at: Utc::now(),
            started_at: None,
            finished_at: None,
            duration_ms: None,
            exit_code: None,
            agent_response_json: None,
            error_code: None,
            error_message: None,
            created_at: Utc::now(),
        };
        let run_value = serde_json::to_value(run).expect("serialize run");
        assert_eq!(run_value["state"], "running");
        assert_eq!(run_value["attempt"], 1);
    }

    #[test]
    fn execution_spec_and_workflow_shapes_are_stable() {
        let spec = ExecutionSpec {
            id: "exec-1".to_string(),
            spec_type: "analysis".to_string(),
            description: "Analyze repository".to_string(),
            input_schema_json: serde_json::json!({
                "type": "object",
                "properties": { "path": { "type": "string" } }
            }),
            output_schema_json: serde_json::json!({
                "type": "object",
                "properties": { "score": { "type": "number" } }
            }),
            artifact_path_template: Some("agentspace/reports/{{date}}/out.md".to_string()),
            skill_refs: vec!["assess-codebase".to_string()],
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let spec_json = serde_json::to_value(spec).expect("serialize spec");
        assert_eq!(spec_json["spec_type"], "analysis");
        assert_eq!(spec_json["is_active"], true);

        let workflow = Workflow {
            id: "wf-1".to_string(),
            name: "Weekly Review".to_string(),
            definition_json: serde_json::json!({
                "steps": [{ "execution_spec_id": "exec-1" }]
            }),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let workflow_json = serde_json::to_value(workflow).expect("serialize workflow");
        assert_eq!(workflow_json["name"], "Weekly Review");
        assert!(workflow_json["definition_json"]["steps"].is_array());
    }

    #[test]
    fn agent_response_envelope_shape_is_stable() {
        let envelope = AgentResponseEnvelope {
            schema_version: 1,
            status: "success".to_string(),
            result: Some(serde_json::json!({ "k": "v" })),
            error: None,
            duration_ms: 1234,
        };
        let value = serde_json::to_value(envelope).expect("serialize envelope");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["status"], "success");
        assert_eq!(value["durationMs"], 1234);
    }
}
