use orbit_types::OrbitError;

use crate::providers::{ClaudeRuntime, CodexRuntime, MockAgentRuntime};
use crate::runtime::AgentRuntime;
use crate::types::{AgentRequest, AgentResponse};

#[allow(clippy::enum_variant_names)]
pub(crate) enum RuntimeBackend {
    CodexCli(CodexRuntime),
    ClaudeCli(ClaudeRuntime),
    MockAgentCli(MockAgentRuntime),
}

impl AgentRuntime for RuntimeBackend {
    fn invoke(&self, req: AgentRequest) -> Result<AgentResponse, OrbitError> {
        match self {
            RuntimeBackend::CodexCli(runtime) => runtime.invoke(req),
            RuntimeBackend::ClaudeCli(runtime) => runtime.invoke(req),
            RuntimeBackend::MockAgentCli(runtime) => runtime.invoke(req),
        }
    }
}
