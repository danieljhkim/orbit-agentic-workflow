use orbit_util::UtilError;
use orbit_util::redaction::redact_sensitive_env_text;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrbitError {
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("task requires approval: {0}")]
    TaskApprovalRequired(String),
    #[error("skill not found: {0}")]
    SkillNotFound(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("job run not found: {0}")]
    JobRunNotFound(String),
    #[error("activity not found: {0}")]
    ActivityNotFound(String),
    #[error("agent session not found: {0}")]
    AgentSessionNotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("skill validation failed: {0}")]
    SkillValidation(String),
    #[error("job validation failed: {0}")]
    JobValidation(String),
    #[error("agent protocol violation: {0}")]
    AgentProtocolViolation(String),
    #[error("unsupported agent provider: {0}")]
    UnsupportedAgentProvider(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("invalid task status transition: {0}")]
    TaskStatusTransition(String),
    #[error("invalid job run state transition: {0}")]
    JobRunStateTransition(String),
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("workspace error: {0}")]
    WorkspaceError(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for OrbitError {
    fn from(err: std::io::Error) -> Self {
        OrbitError::Io(err.to_string())
    }
}

impl From<UtilError> for OrbitError {
    fn from(err: UtilError) -> Self {
        match err {
            UtilError::Io(message) => OrbitError::Io(message),
            UtilError::Execution(message) => OrbitError::Execution(message),
        }
    }
}

/// Apply env-value redaction to the message carried by any `OrbitError` variant.
///
/// Lives here (not in `orbit-util::redaction`) because it operates on the
/// domain error type. `orbit-util` cannot depend on `orbit-types`.
pub fn redact_sensitive_env_error(error: OrbitError) -> OrbitError {
    match error {
        OrbitError::PolicyDenied(m) => OrbitError::PolicyDenied(redact_sensitive_env_text(&m)),
        OrbitError::ToolNotFound(m) => OrbitError::ToolNotFound(redact_sensitive_env_text(&m)),
        OrbitError::TaskNotFound(m) => OrbitError::TaskNotFound(redact_sensitive_env_text(&m)),
        OrbitError::TaskApprovalRequired(m) => {
            OrbitError::TaskApprovalRequired(redact_sensitive_env_text(&m))
        }
        OrbitError::SkillNotFound(m) => OrbitError::SkillNotFound(redact_sensitive_env_text(&m)),
        OrbitError::JobNotFound(m) => OrbitError::JobNotFound(redact_sensitive_env_text(&m)),
        OrbitError::JobRunNotFound(m) => OrbitError::JobRunNotFound(redact_sensitive_env_text(&m)),
        OrbitError::ActivityNotFound(m) => {
            OrbitError::ActivityNotFound(redact_sensitive_env_text(&m))
        }
        OrbitError::AgentSessionNotFound(m) => {
            OrbitError::AgentSessionNotFound(redact_sensitive_env_text(&m))
        }
        OrbitError::InvalidInput(m) => OrbitError::InvalidInput(redact_sensitive_env_text(&m)),
        OrbitError::SkillValidation(m) => {
            OrbitError::SkillValidation(redact_sensitive_env_text(&m))
        }
        OrbitError::JobValidation(m) => OrbitError::JobValidation(redact_sensitive_env_text(&m)),
        OrbitError::AgentProtocolViolation(m) => {
            OrbitError::AgentProtocolViolation(redact_sensitive_env_text(&m))
        }
        OrbitError::UnsupportedAgentProvider(m) => {
            OrbitError::UnsupportedAgentProvider(redact_sensitive_env_text(&m))
        }
        OrbitError::Execution(m) => OrbitError::Execution(redact_sensitive_env_text(&m)),
        OrbitError::Store(m) => OrbitError::Store(redact_sensitive_env_text(&m)),
        OrbitError::TaskStatusTransition(m) => {
            OrbitError::TaskStatusTransition(redact_sensitive_env_text(&m))
        }
        OrbitError::JobRunStateTransition(m) => {
            OrbitError::JobRunStateTransition(redact_sensitive_env_text(&m))
        }
        OrbitError::Io(m) => OrbitError::Io(redact_sensitive_env_text(&m)),
        OrbitError::WorkspaceNotFound(m) => {
            OrbitError::WorkspaceNotFound(redact_sensitive_env_text(&m))
        }
        OrbitError::WorkspaceError(m) => OrbitError::WorkspaceError(redact_sensitive_env_text(&m)),
    }
}
