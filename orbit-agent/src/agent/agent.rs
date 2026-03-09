use orbit_types::OrbitError;

use crate::runtime::{AgentRuntime, RuntimeBackend, resolve_runtime};
use crate::types::{AgentRequest, AgentResponse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentConfig {
    pub command: String,
}

impl AgentConfig {
    pub fn cli(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }
}

pub struct Agent {
    runtime: RuntimeBackend,
}

impl Agent {
    pub fn new(cfg: &AgentConfig) -> Result<Self, OrbitError> {
        Ok(Self {
            runtime: resolve_runtime(cfg)?,
        })
    }

    pub fn invoke(&self, req: AgentRequest) -> Result<AgentResponse, OrbitError> {
        self.runtime.invoke(req)
    }
}
