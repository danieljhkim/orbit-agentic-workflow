use orbit_types::OrbitError;

use crate::providers::AgentProvider;
use crate::providers::mock_agent::mock_agent_cli::MockAgentCliTransport;
use crate::runtime::AgentRuntime;
use crate::types::{AgentRequest, AgentResponse};

pub(crate) struct MockAgentRuntime {
    command: String,
    cli: MockAgentCliTransport,
}

impl MockAgentRuntime {
    pub(crate) fn new(command: String) -> Self {
        Self {
            command,
            cli: MockAgentCliTransport,
        }
    }
}

impl AgentRuntime for MockAgentRuntime {
    fn invoke(&self, req: AgentRequest) -> Result<AgentResponse, OrbitError> {
        Ok(AgentResponse {
            runtime_key: AgentProvider::MockAgent.key(),
            program: self.command.clone(),
            args: self.cli.args(&req.operation),
            stdin: self.cli.stdin(&req.envelope_json),
            stdout_schema_json: None,
            required_env_vars: AgentProvider::MockAgent.required_env_vars(),
        })
    }
}
