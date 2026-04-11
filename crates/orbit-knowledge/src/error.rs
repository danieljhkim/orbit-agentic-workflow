use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[error("{kind}: {reason}")]
pub struct KnowledgeError {
    pub kind: String,
    pub reason: String,
}

impl KnowledgeError {
    pub(crate) fn knowledge_unavailable(reason: impl Into<String>) -> Self {
        Self {
            kind: "knowledge_unavailable".to_string(),
            reason: reason.into(),
        }
    }

    pub(crate) fn invalid_data(reason: impl Into<String>) -> Self {
        Self {
            kind: "knowledge_invalid".to_string(),
            reason: reason.into(),
        }
    }

    pub(crate) fn io(reason: impl Into<String>) -> Self {
        Self {
            kind: "io_error".to_string(),
            reason: reason.into(),
        }
    }
}
