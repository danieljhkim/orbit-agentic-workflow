//! Concrete agent provider implementations and CLI transport helpers.
//!
//! Each provider (`claude`, `codex`, `gemini`, `ollama`, `mock_agent`) translates an [`AgentRequest`]
//! into a CLI command invocation: a program path, argument list, and stdin bytes
//! that the engine will pass to `orbit-exec`. Provider selection is handled by
//! the runtime registry, while each provider module owns the key/env metadata
//! needed to build its invocation spec.
//!
//! The `common` module contains the shared invocation helper used by all
//! providers so that the CLI transport shape stays consistent regardless of
//! which AI backend is selected.

pub(crate) mod claude;
pub(crate) mod codex;
mod common;
pub(crate) mod gemini;
pub(crate) mod mock_agent;
pub(crate) mod ollama;

use crate::types::AgentInvocationSpec;

/// Builds the `AgentInvocationSpec` for a provider, combining CLI args and stdin.
/// All three runtimes share this same structure; only the provider differs.
pub(crate) fn build_invocation_spec(
    runtime_key: &'static str,
    required_env_vars: &'static [&'static str],
    command: String,
    args: Vec<String>,
    stdin: Vec<u8>,
) -> AgentInvocationSpec {
    AgentInvocationSpec {
        runtime_key,
        program: command,
        args,
        stdin,
        stdout_schema_json: None,
        required_env_vars,
    }
}
