use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorType {
    AgentCli,
    DirectAgent,
    CliCommand,
}

impl ExecutorType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AgentCli => "agent_cli",
            Self::DirectAgent => "direct_agent",
            Self::CliCommand => "cli_command",
        }
    }
}

impl fmt::Display for ExecutorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum StdoutFormat {
    Envelope,
    Json,
    Text,
}

impl StdoutFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Envelope => "envelope",
            Self::Json => "json",
            Self::Text => "text",
        }
    }
}

impl fmt::Display for StdoutFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorDef {
    pub name: String,
    /// Executor family, serialized as "agent_cli", "direct_agent", or "cli_command".
    pub executor_type: ExecutorType,
    /// For agent_cli: the CLI command (e.g., "claude", "codex")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    /// Expected stdout format, serialized as "envelope", "json", or "text".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stdout_format: Option<StdoutFormat>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub models: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ExecutorDef {
    pub fn model_for_tier(&self, tier: &str) -> Option<&str> {
        self.models
            .get(tier)
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}
