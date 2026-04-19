//! Support for `orbit serve mcp`.
//!
//! Serves the Orbit tool surface over MCP so external clients can discover and
//! invoke Orbit operations with typed JSON schemas. Each `tools/call` is
//! routed through [`OrbitRuntime::execute_tool_command`], so MCP invocations
//! honor the same policy chain, disabled-tool flag, and audit log as the CLI
//! path. Only stdio transport is supported in this first cut.

use std::sync::Arc;

use clap::Args;
use orbit_common::types::ToolSchema;
use orbit_core::{OrbitError, OrbitRuntime};
use orbit_mcp::McpHost;
use serde_json::Value;

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Serve the Orbit tool registry over Model Context Protocol")]
pub struct ServeArgs {}

impl Execute for ServeArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let host: Arc<dyn McpHost> = Arc::new(RuntimeMcpHost {
            runtime: runtime.clone(),
        });

        let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| OrbitError::Execution(format!("tokio runtime: {e}")))?;

        tokio_runtime.block_on(orbit_mcp::serve_stdio(host))
    }
}

/// [`McpHost`] impl that forwards every MCP operation through the full
/// [`OrbitRuntime`] pipeline.
///
/// Listing is sourced from [`OrbitRuntime::list_tools`], which already filters
/// disabled tools and merges external (non-builtin) entries. Execution is
/// routed through [`OrbitRuntime::execute_tool_command`], which applies the
/// same policy evaluation, workspace sandboxing, and audit event emission as
/// the CLI `orbit tool run` path.
struct RuntimeMcpHost {
    runtime: OrbitRuntime,
}

impl McpHost for RuntimeMcpHost {
    fn list_tool_schemas(&self) -> Vec<ToolSchema> {
        let tools = self.runtime.list_tools().unwrap_or_default();
        tools
            .into_iter()
            .filter(|t| t.enabled)
            .map(|t| ToolSchema {
                name: t.name,
                description: t.description,
                parameters: t.parameters,
                builtin: t.builtin,
            })
            .collect()
    }

    fn call_tool(&self, name: &str, input: Value) -> Result<Value, OrbitError> {
        self.runtime.execute_tool_command(name, input, None, None)
    }
}
