use std::fmt::{Display, Formatter};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Typed identity for attribution across all Orbit artifacts.
///
/// Replaces ad-hoc `(Option<String>, Option<String>)` agent/model pairs.
/// Used in tasks, friction logs, metrics entries, audit trails, and
/// anywhere provenance matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorIdentity {
    /// The Orbit system itself (automated processes with no specific agent).
    System,
    /// An AI agent with a name and model identifier.
    Agent { name: String, model: String },
    /// A human operator.
    Human { label: String },
}

impl ActorIdentity {
    /// Construct an `Agent` variant, trimming whitespace from both fields.
    pub fn agent(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self::Agent {
            name: name.into().trim().to_string(),
            model: model.into().trim().to_string(),
        }
    }

    /// Construct a `Human` variant, trimming whitespace.
    pub fn human(label: impl Into<String>) -> Self {
        Self::Human {
            label: label.into().trim().to_string(),
        }
    }

    /// Build from legacy `(Option<String>, Option<String>)` agent/model pair.
    ///
    /// - Both present → `Agent { name, model }`
    /// - Only agent → `Agent { name, model: "" }` (model unknown)
    /// - Neither → `System`
    pub fn from_legacy(agent: Option<&str>, model: Option<&str>) -> Self {
        match (agent.filter(|s| !s.trim().is_empty()), model.filter(|s| !s.trim().is_empty())) {
            (Some(name), Some(model)) => Self::Agent {
                name: name.trim().to_string(),
                model: model.trim().to_string(),
            },
            (Some(name), None) => Self::Agent {
                name: name.trim().to_string(),
                model: String::new(),
            },
            (None, _) => Self::System,
        }
    }

    /// Returns the agent name if this is an `Agent` variant.
    pub fn agent_name(&self) -> Option<&str> {
        match self {
            Self::Agent { name, .. } => Some(name),
            _ => None,
        }
    }

    /// Returns the model name if this is an `Agent` variant.
    pub fn agent_model(&self) -> Option<&str> {
        match self {
            Self::Agent { model, .. } if !model.is_empty() => Some(model),
            _ => None,
        }
    }

    /// Returns a display label suitable for use in `created_by`, `assigned_to`, etc.
    pub fn label(&self) -> String {
        match self {
            Self::System => "system".to_string(),
            Self::Agent { name, model } if model.is_empty() => name.clone(),
            Self::Agent { name, model } => format!("{name} / {model}"),
            Self::Human { label } => label.clone(),
        }
    }

    /// Returns `true` if this is an `Agent` variant.
    pub fn is_agent(&self) -> bool {
        matches!(self, Self::Agent { .. })
    }

    /// Returns `true` if this is the `System` variant.
    pub fn is_system(&self) -> bool {
        matches!(self, Self::System)
    }

    /// Returns `true` if this is a `Human` variant.
    pub fn is_human(&self) -> bool {
        matches!(self, Self::Human { .. })
    }

    /// Decompose into legacy `(Option<String>, Option<String>)` for backward
    /// compatibility with code that still expects separate agent/model fields.
    pub fn to_legacy(&self) -> (Option<String>, Option<String>) {
        match self {
            Self::System => (None, None),
            Self::Agent { name, model } => (
                Some(name.clone()),
                if model.is_empty() {
                    None
                } else {
                    Some(model.clone())
                },
            ),
            Self::Human { .. } => (None, None),
        }
    }
}

impl Default for ActorIdentity {
    fn default() -> Self {
        Self::System
    }
}

impl Display for ActorIdentity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Custom serialization: produces clean YAML/JSON.
///
/// - `System` → `"system"`
/// - `Agent { name, model }` → `{ "agent": { "name": "...", "model": "..." } }`
/// - `Human { label }` → `{ "human": "..." }`
impl Serialize for ActorIdentity {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::System => serializer.serialize_str("system"),
            Self::Agent { name, model } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(
                    "agent",
                    &AgentFields {
                        name: name.clone(),
                        model: model.clone(),
                    },
                )?;
                map.end()
            }
            Self::Human { label } => {
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry("human", label)?;
                map.end()
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct AgentFields {
    name: String,
    model: String,
}

impl<'de> Deserialize<'de> for ActorIdentity {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct ActorVisitor;

        impl<'de> de::Visitor<'de> for ActorVisitor {
            type Value = ActorIdentity;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str(
                    r#""system", or {"agent": {"name": "...", "model": "..."}}, or {"human": "..."}"#,
                )
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<ActorIdentity, E> {
                match value {
                    "system" => Ok(ActorIdentity::System),
                    other => {
                        // Legacy: bare string like "claude / opus-4.6" or "human"
                        if other.contains(" / ") {
                            let parts: Vec<&str> = other.splitn(2, " / ").collect();
                            Ok(ActorIdentity::Agent {
                                name: parts[0].trim().to_string(),
                                model: parts[1].trim().to_string(),
                            })
                        } else if other == "human" {
                            Ok(ActorIdentity::Human {
                                label: "human".to_string(),
                            })
                        } else {
                            // Could be an agent name without model, or a human label
                            Ok(ActorIdentity::Human {
                                label: other.to_string(),
                            })
                        }
                    }
                }
            }

            fn visit_map<M: de::MapAccess<'de>>(self, mut map: M) -> Result<ActorIdentity, M::Error> {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| de::Error::custom("expected 'agent' or 'human' key"))?;
                match key.as_str() {
                    "agent" => {
                        let fields: AgentFields = map.next_value()?;
                        Ok(ActorIdentity::Agent {
                            name: fields.name,
                            model: fields.model,
                        })
                    }
                    "human" => {
                        let label: String = map.next_value()?;
                        Ok(ActorIdentity::Human { label })
                    }
                    other => Err(de::Error::unknown_field(other, &["agent", "human"])),
                }
            }
        }

        deserializer.deserialize_any(ActorVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_serializes_as_string() {
        let json = serde_json::to_string(&ActorIdentity::System).unwrap();
        assert_eq!(json, r#""system""#);
    }

    #[test]
    fn agent_serializes_as_object() {
        let actor = ActorIdentity::agent("claude", "opus-4.6");
        let json = serde_json::to_string(&actor).unwrap();
        assert_eq!(
            json,
            r#"{"agent":{"name":"claude","model":"opus-4.6"}}"#
        );
    }

    #[test]
    fn human_serializes_as_object() {
        let actor = ActorIdentity::human("daniel");
        let json = serde_json::to_string(&actor).unwrap();
        assert_eq!(json, r#"{"human":"daniel"}"#);
    }

    #[test]
    fn system_round_trips() {
        let actor = ActorIdentity::System;
        let json = serde_json::to_string(&actor).unwrap();
        let decoded: ActorIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, actor);
    }

    #[test]
    fn agent_round_trips() {
        let actor = ActorIdentity::agent("codex", "gpt-5.4");
        let json = serde_json::to_string(&actor).unwrap();
        let decoded: ActorIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, actor);
    }

    #[test]
    fn human_round_trips() {
        let actor = ActorIdentity::human("daniel");
        let json = serde_json::to_string(&actor).unwrap();
        let decoded: ActorIdentity = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, actor);
    }

    #[test]
    fn legacy_string_deserializes_agent_with_model() {
        let decoded: ActorIdentity =
            serde_json::from_str(r#""claude / opus-4.6""#).unwrap();
        assert_eq!(
            decoded,
            ActorIdentity::Agent {
                name: "claude".to_string(),
                model: "opus-4.6".to_string(),
            }
        );
    }

    #[test]
    fn legacy_string_deserializes_human() {
        let decoded: ActorIdentity = serde_json::from_str(r#""human""#).unwrap();
        assert_eq!(
            decoded,
            ActorIdentity::Human {
                label: "human".to_string()
            }
        );
    }

    #[test]
    fn from_legacy_both_present() {
        let actor = ActorIdentity::from_legacy(Some("claude"), Some("opus-4.6"));
        assert_eq!(actor, ActorIdentity::agent("claude", "opus-4.6"));
    }

    #[test]
    fn from_legacy_agent_only() {
        let actor = ActorIdentity::from_legacy(Some("claude"), None);
        assert_eq!(
            actor,
            ActorIdentity::Agent {
                name: "claude".to_string(),
                model: String::new(),
            }
        );
    }

    #[test]
    fn from_legacy_none() {
        let actor = ActorIdentity::from_legacy(None, None);
        assert_eq!(actor, ActorIdentity::System);
    }

    #[test]
    fn label_formatting() {
        assert_eq!(ActorIdentity::System.label(), "system");
        assert_eq!(
            ActorIdentity::agent("claude", "opus-4.6").label(),
            "claude / opus-4.6"
        );
        assert_eq!(ActorIdentity::human("daniel").label(), "daniel");
    }

    #[test]
    fn to_legacy_round_trips() {
        let (agent, model) = ActorIdentity::agent("claude", "opus-4.6").to_legacy();
        assert_eq!(agent.as_deref(), Some("claude"));
        assert_eq!(model.as_deref(), Some("opus-4.6"));

        let (agent, model) = ActorIdentity::System.to_legacy();
        assert_eq!(agent, None);
        assert_eq!(model, None);
    }

    // YAML round-trip tests live in orbit-store which has serde_yaml as a dependency.
}
