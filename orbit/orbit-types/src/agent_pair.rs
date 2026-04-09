//! Authoritative mapping from an agent CLI family to the orchestrator/helper
//! model pair that should drive bounded implementation work.
//!
//! This is the single source of truth Orbit consults whenever an activity needs
//! to embed a model duo into its instructions. Splitting the heavy "judgment"
//! model from a cheaper "implementation" helper makes execution mode
//! deterministic per agent family rather than depending on per-prompt edits.
//!
//! Activities reference the resolved pair via the `{{orchestrator_model}}`,
//! `{{helper_model}}`, and `{{agent_family}}` placeholders, which the runtime
//! substitutes into the instruction text before invoking the agent.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// A resolved (orchestrator, helper) duo for a given agent family.
///
/// - `orchestrator` owns plan, review, and integration responsibilities.
/// - `helper` owns the bounded implementation work delegated by the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelPair {
    pub orchestrator: String,
    pub helper: String,
}

impl AgentModelPair {
    pub fn new(orchestrator: impl Into<String>, helper: impl Into<String>) -> Self {
        Self {
            orchestrator: orchestrator.into(),
            helper: helper.into(),
        }
    }
}

/// Normalize an `agent_cli` value into a stable, lowercased family identifier
/// (e.g. `/usr/local/bin/Codex` -> `codex`).
pub fn agent_family_from_cli(agent_cli: &str) -> String {
    Path::new(agent_cli)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(agent_cli)
        .to_ascii_lowercase()
}

/// Resolve the orchestrator/helper model pair for an `agent_cli`.
///
/// Returns `None` for unknown agent families. Callers that need a fallback
/// (for example, the runtime envelope renderer) decide what placeholder text
/// to inject when no mapping is registered.
pub fn resolve_agent_model_pair(agent_cli: &str) -> Option<AgentModelPair> {
    let family = agent_family_from_cli(agent_cli);
    match family.as_str() {
        "codex" => Some(AgentModelPair::new("gpt-5.4", "gpt-5.4-mini")),
        "claude" => Some(AgentModelPair::new("opus", "sonnet")),
        "gemini" => Some(AgentModelPair::new("gemini-3.1-pro", "gemini-3-flash")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_normalizes_path_and_case() {
        assert_eq!(agent_family_from_cli("codex"), "codex");
        assert_eq!(agent_family_from_cli("/usr/local/bin/Codex"), "codex");
        assert_eq!(agent_family_from_cli("/opt/CLAUDE"), "claude");
    }

    #[test]
    fn resolves_codex_pair() {
        let pair = resolve_agent_model_pair("codex").expect("codex pair");
        assert_eq!(pair.orchestrator, "gpt-5.4");
        assert_eq!(pair.helper, "gpt-5.4-mini");
    }

    #[test]
    fn resolves_claude_pair() {
        let pair = resolve_agent_model_pair("claude").expect("claude pair");
        assert_eq!(pair.orchestrator, "opus");
        assert_eq!(pair.helper, "sonnet");
    }

    #[test]
    fn resolves_gemini_pair() {
        let pair = resolve_agent_model_pair("gemini").expect("gemini pair");
        assert_eq!(pair.orchestrator, "gemini-3.1-pro");
        assert_eq!(pair.helper, "gemini-3-flash");
    }

    #[test]
    fn resolves_path_prefixed_cli() {
        let pair =
            resolve_agent_model_pair("/usr/local/bin/codex").expect("path-prefixed codex pair");
        assert_eq!(pair.orchestrator, "gpt-5.4");
        assert_eq!(pair.helper, "gpt-5.4-mini");
    }

    #[test]
    fn unknown_agent_returns_none() {
        assert!(resolve_agent_model_pair("mock-agent").is_none());
        assert!(resolve_agent_model_pair("").is_none());
    }
}
