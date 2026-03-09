use orbit_types::OrbitError;

use crate::providers::AgentProvider;
use crate::providers::codex::codex_cli::CodexCliTransport;
use crate::runtime::AgentRuntime;
use crate::types::{AgentRequest, AgentResponse};

pub(crate) struct CodexRuntime {
    command: String,
    cli: CodexCliTransport,
}

impl CodexRuntime {
    pub(crate) fn new(command: String) -> Self {
        Self {
            command,
            cli: CodexCliTransport,
        }
    }
}

impl AgentRuntime for CodexRuntime {
    fn invoke(&self, req: AgentRequest) -> Result<AgentResponse, OrbitError> {
        Ok(AgentResponse {
            runtime_key: AgentProvider::Codex.key(),
            program: self.command.clone(),
            args: self.cli.args(&req.operation),
            stdin: self.cli.stdin(&req.envelope_json),
            required_env_vars: AgentProvider::Codex.required_env_vars(),
        })
    }
}
