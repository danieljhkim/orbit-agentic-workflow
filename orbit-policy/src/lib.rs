use std::collections::HashSet;

use orbit_types::PolicyDecision;

#[derive(Debug, Clone, Default)]
pub struct PolicyContext {
    pub entrypoint: String,
    pub tool_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    default_allow: bool,
    denied_tools: HashSet<String>,
}

impl PolicyEngine {
    pub fn new_local_default_allow() -> Self {
        Self {
            default_allow: true,
            denied_tools: HashSet::new(),
        }
    }

    pub fn deny_tool(mut self, name: impl Into<String>) -> Self {
        self.denied_tools.insert(name.into());
        self
    }

    pub fn evaluate(&self, ctx: &PolicyContext) -> PolicyDecision {
        if let Some(tool_name) = &ctx.tool_name
            && self.denied_tools.contains(tool_name)
        {
            return PolicyDecision::Deny {
                reason: format!("tool `{tool_name}` denied by policy"),
            };
        }

        if self.default_allow {
            PolicyDecision::Allow
        } else {
            PolicyDecision::Deny {
                reason: "default deny policy".to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_by_default_for_local_runtime() {
        let engine = PolicyEngine::new_local_default_allow();
        let decision = engine.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some("fs.read".to_string()),
        });

        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn denies_explicitly_blocked_tool() {
        let engine = PolicyEngine::new_local_default_allow().deny_tool("fs.read");
        let decision = engine.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some("fs.read".to_string()),
        });

        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }
}
