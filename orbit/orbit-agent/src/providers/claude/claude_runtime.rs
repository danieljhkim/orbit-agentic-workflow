use orbit_types::{InvocationTrace, OrbitError};

use crate::providers::claude::claude_cli::ClaudeCliTransport;
use crate::providers::{AgentProvider, build_agent_response};
use crate::runtime::AgentRuntime;
use crate::types::{AgentRequest, AgentResponse};

pub(crate) struct ClaudeRuntime {
    command: String,
    cli: ClaudeCliTransport,
}

impl ClaudeRuntime {
    pub(crate) fn new(command: String, model: Option<String>) -> Self {
        Self {
            command,
            cli: ClaudeCliTransport::new(model),
        }
    }
}

impl AgentRuntime for ClaudeRuntime {
    fn invoke(&self, req: AgentRequest) -> Result<(AgentResponse, InvocationTrace), OrbitError> {
        Ok((
            build_agent_response(
                AgentProvider::Claude,
                self.command.clone(),
                self.cli.args(req.verbose),
                self.cli.stdin(&req.envelope_json),
            ),
            InvocationTrace::default(),
        ))
    }

    fn model_name(&self) -> Option<&str> {
        self.cli.model_name()
    }
}
