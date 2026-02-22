use orbit_policy::PolicyContext;
use orbit_tools::ToolContext;
use orbit_types::{OrbitEvent, PolicyDecision};
use serde_json::Value;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn run_tool(&self, name: &str, input: Value) -> Result<Value, OrbitError> {
        let decision = self.context.policy.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some(name.to_string()),
        });

        match decision {
            PolicyDecision::Deny { reason } => {
                self.with_mutation(|_| {
                    Ok((
                        (),
                        OrbitEvent::PolicyDenied {
                            tool: name.to_string(),
                        },
                    ))
                })?;
                Err(OrbitError::PolicyDenied(reason))
            }
            PolicyDecision::Allow => {
                let output = self
                    .context
                    .registry
                    .execute(name, &ToolContext::default(), input)?;

                self.with_mutation(|_| {
                    Ok((
                        (),
                        OrbitEvent::ToolExecuted {
                            name: name.to_string(),
                        },
                    ))
                })?;

                Ok(output)
            }
        }
    }
}
