#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentOperation {
    Scheduled {
        target_type: String,
        target_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    pub operation: AgentOperation,
    pub envelope_json: Vec<u8>,
}

impl AgentRequest {
    pub fn scheduled(
        target_type: impl Into<String>,
        target_id: impl Into<String>,
        envelope_json: Vec<u8>,
    ) -> Self {
        Self {
            operation: AgentOperation::Scheduled {
                target_type: target_type.into(),
                target_id: target_id.into(),
            },
            envelope_json,
        }
    }
}
