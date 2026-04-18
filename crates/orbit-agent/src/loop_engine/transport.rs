//! Provider-agnostic HTTP transport contract for the agent loop.
//!
//! Each provider family (Anthropic Messages, OpenAI chat-completions, Gemini
//! generateContent) implements [`LoopTransport`]. The trait is deliberately
//! shaped around the most expressive wire format (Anthropic content blocks
//! and `cache_control`), so collapsing OpenAI-compat or Gemini into it does
//! not lose fidelity. Keeping the trait here — sibling to the existing
//! `AgentRuntime` — satisfies the plan's two-traits-coexist rule.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
}

impl Message {
    pub fn user_text(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![ContentBlock::Text { text: text.into() }],
        }
    }

    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: blocks,
        }
    }

    pub fn user_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content: blocks,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Copy)]
pub enum CacheHint {
    None,
    SystemOnly,
    SystemAndEarliestHistory,
}

#[derive(Debug)]
pub struct TurnRequest<'a> {
    pub system: Option<&'a str>,
    pub messages: &'a [Message],
    pub tools: &'a [ToolSpec],
    pub cache_hint: CacheHint,
    pub max_response_tokens: u32,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    ToolUse,
    MaxTokens,
    Other,
}

impl StopReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::ToolUse => "tool_use",
            StopReason::MaxTokens => "max_tokens",
            StopReason::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TurnUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub cache_creation_input_tokens: u64,
}

#[derive(Debug)]
pub struct TurnResponse {
    pub content: Vec<ContentBlock>,
    pub stop_reason: StopReason,
    pub usage: TurnUsage,
    pub raw_request_body: Vec<u8>,
    pub raw_response_body: Vec<u8>,
    pub endpoint: String,
    pub http_status: u16,
}

#[derive(Debug)]
pub enum TransportError {
    Network(String),
    BadStatus { status: u16, body: String },
    Decode(String),
    Auth(String),
    Other(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::Network(msg) => write!(f, "transport network error: {msg}"),
            TransportError::BadStatus { status, body } => {
                write!(f, "transport bad status {status}: {body}")
            }
            TransportError::Decode(msg) => write!(f, "transport decode error: {msg}"),
            TransportError::Auth(msg) => write!(f, "transport auth error: {msg}"),
            TransportError::Other(msg) => write!(f, "transport error: {msg}"),
        }
    }
}

impl std::error::Error for TransportError {}

pub trait LoopTransport: Send + Sync {
    fn provider(&self) -> &str;
    fn model(&self) -> &str;
    fn send_turn(&self, req: &TurnRequest<'_>) -> Result<TurnResponse, TransportError>;
}
