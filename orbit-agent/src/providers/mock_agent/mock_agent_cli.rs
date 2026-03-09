use crate::types::AgentOperation;

pub(crate) struct MockAgentCliTransport;

impl MockAgentCliTransport {
    pub(crate) fn args(&self, operation: &AgentOperation) -> Vec<String> {
        match operation {
            AgentOperation::Scheduled {
                target_type,
                target_id,
            } => vec![
                "run".to_string(),
                "--target-type".to_string(),
                target_type.clone(),
                "--target-id".to_string(),
                target_id.clone(),
                "--mode".to_string(),
                "scheduled".to_string(),
                "--output".to_string(),
                "json".to_string(),
            ],
        }
    }

    pub(crate) fn stdin(&self, envelope_json: &[u8]) -> Vec<u8> {
        envelope_json.to_vec()
    }
}
