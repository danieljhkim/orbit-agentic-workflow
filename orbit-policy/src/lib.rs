//! RBAC policy evaluation engine for Orbit tool and command authorization.
//!
//! Evaluates whether a given actor (identified by [`orbit_types::Role`]) is
//! permitted to invoke a specific tool or CLI entrypoint, based on a set of
//! configured allow/deny rules.
//!
//! # Role
//! Sits directly above `orbit-types` in the dependency graph. Consumed by
//! `orbit-core`, which wires the [`PolicyEngine`] into the runtime so that
//! every tool call passes through policy evaluation before execution.
//!
//! # Key exports
//! - [`PolicyEngine`] — stateful evaluator; constructed with allow/deny rule sets
//! - [`PolicyContext`] — per-call context (entrypoint, tool name, role)
//! - [`PolicyDecision`] — `Allow` or `Deny { reason }` result
//!
//! # Dependency direction
//! `orbit-types` → `orbit-policy` → orbit-core

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
