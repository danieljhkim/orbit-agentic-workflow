use orbit_types::OrbitError;

use crate::types::{AgentRequest, AgentResponse};

pub trait AgentRuntime {
    fn invoke(&self, req: AgentRequest) -> Result<AgentResponse, OrbitError>;
}
