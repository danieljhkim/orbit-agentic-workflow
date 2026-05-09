use orbit_common::types::{
    AgentModelPair, OrbitError, agent_family_from_cli, normalize_agent_family_for_model,
    resolve_agent_model_pair,
};

use crate::OrbitRuntime;

pub(super) fn normalize_agent_name(agent_cli: &str) -> String {
    std::path::Path::new(agent_cli)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(agent_cli)
        .to_ascii_lowercase()
}

impl OrbitRuntime {
    pub(crate) fn configured_agent_model_pair(&self, agent_cli: &str) -> Option<AgentModelPair> {
        self.stores()
            .executors()
            .get(agent_cli)
            .ok()
            .flatten()
            .and_then(|def| {
                let orchestrator = normalize_configured_model_for_agent(
                    agent_cli,
                    def.model_for_tier("strong")?,
                    true,
                );
                let helper = normalize_configured_model_for_agent(
                    agent_cli,
                    def.model_for_tier("weak")?,
                    false,
                );
                Some(AgentModelPair::new(orchestrator, helper))
            })
            .or_else(|| resolve_agent_model_pair(agent_cli))
    }

    pub(crate) fn canonical_model_for_agent(
        &self,
        agent_cli: &str,
        model: Option<&str>,
    ) -> Option<String> {
        let requested = model.map(str::trim).filter(|value| !value.is_empty())?;
        let pair = self.configured_agent_model_pair(agent_cli);
        let family = agent_family_from_cli(agent_cli);

        if requested.eq_ignore_ascii_case("strong") {
            return pair.map(|pair| pair.orchestrator);
        }
        if requested.eq_ignore_ascii_case("weak") {
            return pair.map(|pair| pair.helper);
        }

        if let Some(pair) = pair {
            if requested.eq_ignore_ascii_case(&pair.orchestrator) {
                return Some(pair.orchestrator);
            }
            if requested.eq_ignore_ascii_case(&pair.helper) {
                return Some(pair.helper);
            }
            if matches_model_alias(&family, requested, &pair.orchestrator, true) {
                return Some(pair.orchestrator);
            }
            if matches_model_alias(&family, requested, &pair.helper, false) {
                return Some(pair.helper);
            }
        }

        Some(requested.to_string())
    }

    pub(crate) fn canonical_agent_model_identity(
        &self,
        agent_cli: Option<&str>,
        model: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        self.try_canonical_agent_model_identity(agent_cli, model)
            .unwrap_or_else(|_| self.legacy_canonical_agent_model_identity(agent_cli, model))
    }

    pub(crate) fn try_canonical_agent_model_identity(
        &self,
        agent_cli: Option<&str>,
        model: Option<&str>,
    ) -> Result<(Option<String>, Option<String>), OrbitError> {
        let agent = normalize_agent_family_for_model(agent_cli, model)?;
        let requested_model = model.map(str::trim).filter(|value| !value.is_empty());
        let model = agent
            .as_deref()
            .and_then(|agent| self.canonical_model_for_agent(agent, requested_model))
            .or_else(|| requested_model.map(ToOwned::to_owned));
        Ok((agent, model))
    }

    fn legacy_canonical_agent_model_identity(
        &self,
        agent_cli: Option<&str>,
        model: Option<&str>,
    ) -> (Option<String>, Option<String>) {
        let agent = agent_cli
            .map(normalize_agent_name)
            .filter(|value| !value.trim().is_empty());
        let model = agent
            .as_deref()
            .and_then(|agent| self.canonical_model_for_agent(agent, model))
            .or_else(|| {
                model
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToOwned::to_owned)
            });
        (agent, model)
    }
}

fn matches_model_alias(family: &str, requested: &str, configured: &str, strong: bool) -> bool {
    if requested.eq_ignore_ascii_case(configured) {
        return true;
    }

    if let Some(default_pair) = resolve_agent_model_pair(family) {
        let fallback = if strong {
            default_pair.orchestrator
        } else {
            default_pair.helper
        };
        if requested.eq_ignore_ascii_case(&fallback) {
            return true;
        }
    }

    match (family, strong) {
        ("claude", true) => {
            requested.eq_ignore_ascii_case("opus")
                || claude_cli_full_model_name(configured)
                    .is_some_and(|value| requested.eq_ignore_ascii_case(&value))
        }
        ("claude", false) => {
            requested.eq_ignore_ascii_case("sonnet")
                || claude_cli_full_model_name(configured)
                    .is_some_and(|value| requested.eq_ignore_ascii_case(&value))
        }
        ("gemini", true) => ["gemini-3.1-pro", "gemini-3-pro", "gemini-3-pro-preview"]
            .iter()
            .any(|alias| requested.eq_ignore_ascii_case(alias)),
        ("gemini", false) => requested.eq_ignore_ascii_case("gemini-3-flash"),
        _ => false,
    }
}

fn normalize_configured_model_for_agent(agent_cli: &str, model: &str, strong: bool) -> String {
    let family = agent_family_from_cli(agent_cli);
    let trimmed = model.trim();
    let lower = trimmed.to_ascii_lowercase();

    match (family.as_str(), strong, lower.as_str()) {
        ("gemini", true, "gemini-3.1-pro" | "gemini-3-pro" | "gemini-3-pro-preview") => {
            "gemini-3.1-pro-preview".to_string()
        }
        ("gemini", false, "gemini-3-flash") => "gemini-3-flash-preview".to_string(),
        _ => trimmed.to_string(),
    }
}

fn claude_cli_full_model_name(model: &str) -> Option<String> {
    let trimmed = model.trim();
    if let Some(version) = trimmed.strip_prefix("opus-") {
        return Some(format!("claude-opus-{}", version.replace('.', "-")));
    }
    if let Some(version) = trimmed.strip_prefix("sonnet-") {
        return Some(format!("claude-sonnet-{}", version.replace('.', "-")));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use orbit_common::types::{ExecutorDef, ExecutorType};
    use std::collections::HashMap;
    use tempfile::{TempDir, tempdir};

    fn test_runtime() -> (TempDir, OrbitRuntime) {
        let root = tempdir().expect("tempdir");
        let global = root.path().join("home/.orbit");
        let workspace = root.path().join("repo/.orbit");
        std::fs::create_dir_all(&global).expect("create global root");
        std::fs::create_dir_all(&workspace).expect("create workspace root");
        let runtime = OrbitRuntime::from_roots(&global, &workspace).expect("build runtime");
        (root, runtime)
    }

    #[test]
    fn gemini_defaults_use_current_preview_model_ids() {
        let (_root, runtime) = test_runtime();

        let pair = runtime
            .configured_agent_model_pair("gemini")
            .expect("gemini pair");

        assert_eq!(pair.orchestrator, "gemini-3.1-pro-preview");
        assert_eq!(pair.helper, "gemini-3-flash-preview");
    }

    #[test]
    fn stale_gemini_executor_models_are_canonicalized() {
        let (_root, runtime) = test_runtime();
        let now = Utc::now();
        runtime
            .upsert_executor_def(&ExecutorDef {
                name: "gemini".to_string(),
                executor_type: ExecutorType::DirectAgent,
                command: Some("gemini".to_string()),
                args: Vec::new(),
                stdout_format: None,
                models: HashMap::from([
                    ("strong".to_string(), "gemini-3.1-pro".to_string()),
                    ("weak".to_string(), "gemini-3-flash".to_string()),
                ]),
                timeout_seconds: None,
                env: HashMap::new(),
                sandbox: None,
                allow_fallback: false,
                created_at: now,
                updated_at: now,
            })
            .expect("seed stale executor");

        let pair = runtime
            .configured_agent_model_pair("gemini")
            .expect("gemini pair");
        assert_eq!(pair.orchestrator, "gemini-3.1-pro-preview");
        assert_eq!(pair.helper, "gemini-3-flash-preview");
        assert_eq!(
            runtime.canonical_model_for_agent("gemini", Some("gemini-3.1-pro")),
            Some("gemini-3.1-pro-preview".to_string())
        );
        assert_eq!(
            runtime.canonical_model_for_agent("gemini", Some("gemini-3-pro")),
            Some("gemini-3.1-pro-preview".to_string())
        );
        assert_eq!(
            runtime.canonical_model_for_agent("gemini", Some("weak")),
            Some("gemini-3-flash-preview".to_string())
        );
    }
}
