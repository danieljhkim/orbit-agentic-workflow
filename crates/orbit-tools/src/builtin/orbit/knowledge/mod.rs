#[allow(dead_code)]
mod add;
pub mod callers;
#[allow(dead_code)]
mod delete;
pub mod deps;
pub mod implementors;
#[allow(dead_code)]
mod move_;
pub mod overview;
pub mod pack;
pub mod refs;
pub mod search;
pub mod show;
#[allow(dead_code)]
mod write;

use orbit_common::types::OrbitError;
use orbit_knowledge::TaskGraphService;
use orbit_knowledge::graph::nodes::CodebaseGraphV1;
use serde_json::Value;

use crate::ToolContext;

pub(super) fn has_explicit_knowledge_dir(input: &Value) -> bool {
    input
        .get("knowledge_dir")
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}

pub(super) fn load_graph_for_read(
    ctx: &ToolContext,
    input: &Value,
) -> Result<CodebaseGraphV1, OrbitError> {
    let knowledge_dir = write::resolve_knowledge_dir(ctx, input)?;
    let service = TaskGraphService::new(knowledge_dir, write::task_graph_scope(ctx));
    let explicit_ref = super::optional_string(input, "ref")?;
    service.read_graph(
        ctx.workspace_root.as_deref(),
        has_explicit_knowledge_dir(input),
        explicit_ref.as_deref(),
    )
}
