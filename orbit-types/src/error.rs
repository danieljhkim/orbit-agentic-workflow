use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrbitError {
    #[error("policy denied: {0}")]
    PolicyDenied(String),
    #[error("tool not found: {0}")]
    ToolNotFound(String),
    #[error("task not found: {0}")]
    TaskNotFound(String),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("execution failed: {0}")]
    Execution(String),
    #[error("store error: {0}")]
    Store(String),
    #[error("io error: {0}")]
    Io(String),
}
