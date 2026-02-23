use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrbitError {
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("skill not found: {0}")]
    SkillNotFound(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("job run not found: {0}")]
    JobRunNotFound(String),
    #[error("execution spec not found: {0}")]
    ExecutionSpecNotFound(String),
    #[error("workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("agent session not found: {0}")]
    AgentSessionNotFound(String),
    #[error("entry not found: {0}")]
    EntryNotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("skill validation failed: {0}")]
    SkillValidation(String),
    #[error("job validation failed: {0}")]
    JobValidation(String),
    #[error("agent run failed: {0}")]
    AgentRun(String),
    #[error("agent protocol violation: {0}")]
    AgentProtocolViolation(String),
    #[error("unsupported agent provider: {0}")]
    UnsupportedAgentProvider(String),
    #[error("entry validation failed: {0}")]
    EntryValidation(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("io error: {0}")]
    Io(String),
}
