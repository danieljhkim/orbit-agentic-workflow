//! Agent provider abstraction for Orbit. Two transport families coexist:
//!
//! - **CLI transports** — drive `claude`, `codex`, `gemini`, `ollama`, or
//!   `mock` as subprocesses through [`AgentRuntime`]. Each runtime builds an
//!   [`AgentInvocationSpec`] (program, args, stdin envelope) that the engine
//!   executes through `orbit-exec`; responses are parsed via
//!   [`parse_and_validate_response`].
//! - **HTTP transports** — drive providers directly through the
//!   [`LoopTransport`](loop_engine::LoopTransport) sibling trait. The
//!   provider-agnostic [`AgentLoop`](loop_engine::AgentLoop) runs the
//!   send/parse/dispatch cycle, enforcing guardrails and tool-allowlist rules
//!   and emitting the full structured audit trail via
//!   [`AuditSink`](loop_engine::AuditSink).
//!
//! The two trait shapes differ intentionally — one-shot command descriptor
//! vs. iterative conversation driver — so they coexist instead of being
//! forcibly unified. The CLI path is unchanged by this module's introduction;
//! existing activities keep working.
//!
//! # Role
//! Depends on `orbit-types` (shared domain types) and `orbit-tools`
//! (`ToolRegistry` dispatch for HTTP-loop tool calls). Consumed by
//! `orbit-engine`.
//!
//! # Key exports
//! - [`AgentRuntime`] trait and CLI [`Agent`] / [`AgentConfig`] wrappers
//! - [`parse_and_validate_response`] for CLI response envelopes
//! - [`loop_engine::AgentLoop`], [`loop_engine::Session`],
//!   [`loop_engine::LoopTransport`], [`loop_engine::LoopAuditEvent`],
//!   [`loop_engine::AuditSink`] for the HTTP path
//! - [`providers::anthropic::AnthropicMessagesTransport`] — Anthropic HTTP
//!   transport (Phase 1). OpenAI-compat and Gemini HTTP transports land in
//!   follow-up tasks.
//!
//! # Dependency direction
//! `orbit-types` / `orbit-tools` → `orbit-agent` → `orbit-engine`

mod agent;
pub mod loop_engine;
pub mod providers;
mod runtime;
mod types;

pub use agent::{Agent, AgentConfig, ProviderOptions};
pub use orbit_types::{InvocationTrace, TokenUsage, ToolCallTrace};
pub use runtime::AgentRuntime;
pub use types::{AgentInvocationSpec, AgentOperation, AgentRequest, AgentResponseStatus};
pub use types::{is_timeout, parse_and_validate_response};
