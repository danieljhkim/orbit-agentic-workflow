use crate::providers::common::render_prompt_with_embedded_envelope;
use crate::types::AgentOperation;

pub(crate) struct ClaudeCliTransport;

impl ClaudeCliTransport {
    pub(crate) fn args(&self, _operation: &AgentOperation) -> Vec<String> {
        vec![
            "-p".to_string(),
            "--permission-mode".to_string(),
            "bypassPermissions".to_string(),
            "--output-format".to_string(),
            "text".to_string(),
            "--no-session-persistence".to_string(),
        ]
    }

    pub(crate) fn stdin(&self, envelope_json: &[u8]) -> Vec<u8> {
        render_prompt_with_embedded_envelope(envelope_json)
    }
}
