mod claude;
mod codex;
mod common;
mod mock_agent;

use std::path::Path;

use orbit_types::OrbitError;

pub(crate) use claude::ClaudeRuntime;
pub(crate) use codex::CodexRuntime;
pub(crate) use mock_agent::MockAgentRuntime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AgentProvider {
    MockAgent,
    Codex,
    Claude,
}

impl AgentProvider {
    pub(crate) fn key(self) -> &'static str {
        match self {
            AgentProvider::MockAgent => "mock-agent",
            AgentProvider::Codex => "codex",
            AgentProvider::Claude => "claude",
        }
    }

    pub(crate) fn required_env_vars(self) -> &'static [&'static str] {
        match self {
            AgentProvider::MockAgent => &[],
            AgentProvider::Codex => &["HOME", "PATH"],
            AgentProvider::Claude => &["HOME", "PATH", "ANTHROPIC_API_KEY"],
        }
    }

    pub(crate) fn detect_from_cli(agent_cli: &str) -> Result<Self, OrbitError> {
        match provider_key(agent_cli).as_str() {
            "mock-agent" => Ok(AgentProvider::MockAgent),
            "codex" => Ok(AgentProvider::Codex),
            "claude" => Ok(AgentProvider::Claude),
            other => Err(OrbitError::UnsupportedAgentProvider(other.to_string())),
        }
    }
}

fn provider_key(agent_cli: &str) -> String {
    Path::new(agent_cli)
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_else(|| agent_cli.to_ascii_lowercase())
}
