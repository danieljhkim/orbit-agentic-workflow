use orbit_common::types::OrbitError;
use orbit_engine::{AgentProtocolHost, ExecutionContext};

use super::envelope::build_agent_stdin_envelope_payload;
use crate::OrbitRuntime;

impl AgentProtocolHost for OrbitRuntime {
    fn build_agent_stdin_envelope_payload(
        &self,
        execution: &ExecutionContext,
    ) -> Result<Vec<u8>, OrbitError> {
        build_agent_stdin_envelope_payload(self, execution)
    }
}
