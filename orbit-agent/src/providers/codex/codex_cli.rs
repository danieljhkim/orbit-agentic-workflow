use crate::providers::common::render_prompt_with_embedded_envelope;
use crate::types::AgentOperation;

pub(crate) struct CodexCliTransport;

impl CodexCliTransport {
    pub(crate) fn args(&self, _operation: &AgentOperation) -> Vec<String> {
        vec![
            "exec".to_string(),
            "--sandbox".to_string(),
            "workspace-write".to_string(),
        ]
    }

    pub(crate) fn stdin(&self, envelope_json: &[u8]) -> Vec<u8> {
        render_prompt_with_embedded_envelope(envelope_json)
    }
}
