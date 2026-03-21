pub mod decision;
pub mod engine;
mod evaluator;

pub use decision::PolicyDecision;
pub use engine::{PolicyContext, PolicyEngine};

#[cfg(test)]
mod tests {
    use crate::{PolicyContext, PolicyDecision, PolicyEngine};
    use orbit_types::Role;

    #[test]
    fn allows_by_default_for_local_runtime() {
        let engine = PolicyEngine::new_local_default_allow();
        let decision = engine.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some("fs.read".to_string()),
            role: Role::Admin,
        });

        assert_eq!(decision, PolicyDecision::Allow);
    }

    #[test]
    fn denies_explicitly_blocked_tool() {
        let engine = PolicyEngine::new_local_default_allow().deny_tool("fs.read");
        let decision = engine.evaluate(&PolicyContext {
            entrypoint: "cli".to_string(),
            tool_name: Some("fs.read".to_string()),
            role: Role::Admin,
        });

        assert!(matches!(decision, PolicyDecision::Deny { .. }));
    }
}
