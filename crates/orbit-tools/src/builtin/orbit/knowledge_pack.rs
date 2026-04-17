use std::path::PathBuf;

use orbit_knowledge::{
    KnowledgeError, KnowledgeStore, Selector, load_task_working_graph,
    overlay_pack_with_working_graph, pack_from_working_graph,
};
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::Value;

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgePackTool;

impl Tool for OrbitKnowledgePackTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.pack".to_string(),
            description:
                "Resolve knowledge selectors into a scoped pack from `.orbit/knowledge` artifacts. `file:` selectors return file metadata and symbol summaries, not full file source."
                    .to_string(),
            parameters: vec![
                ToolParam {
                    name: "selectors".to_string(),
                    description: "Selector strings like `file:path`, `symbol:path#symbol:kind`, or `dir:path`. Use `orbit.graph.show` or `symbol:` selectors when you need file source.".to_string(),
                    param_type: "array".to_string(),
                    required: true,
                },
                ToolParam {
                    name: "knowledge_dir".to_string(),
                    description: "Optional knowledge artifact directory; defaults to `<workspace>/.orbit/knowledge`".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let selectors = parse_selector_strings(&input)?;
        let selectors = Selector::parse_many(&selectors)
            .map_err(|error| OrbitError::InvalidInput(error.to_string()))?;
        let knowledge_dir = resolve_knowledge_dir(ctx, &input)?;
        super::maybe_refresh_knowledge_graph(ctx, &input, &knowledge_dir);

        let working_graph =
            load_task_working_graph(ctx.orbit_root.as_deref(), ctx.task_id.as_deref())?;

        let pack_result = || -> Result<_, KnowledgeError> {
            let store = KnowledgeStore::open(&knowledge_dir)?;
            store.pack(&selectors)
        };
        let pack = match pack_result() {
            Ok(pack) => pack,
            Err(first_error) => {
                let pack_or_error = match super::rebuild_default_knowledge_graph(
                    ctx,
                    &knowledge_dir,
                    &first_error,
                ) {
                    Ok(true) => match pack_result() {
                        Ok(pack) => Ok(pack),
                        Err(retry_error) => Err(KnowledgeError {
                            kind: "knowledge_unavailable".to_string(),
                            reason: format!(
                                "failed to load knowledge pack: {first_error}; retry after rebuild failed: {retry_error}"
                            ),
                        }),
                    },
                    Ok(false) => Err(first_error),
                    Err(rebuild_error) => Err(KnowledgeError {
                        kind: "knowledge_unavailable".to_string(),
                        reason: format!(
                            "failed to load knowledge pack: {first_error}; rebuild attempt failed: {rebuild_error}"
                        ),
                    }),
                };

                match pack_or_error {
                    Ok(pack) => pack,
                    Err(error) => {
                        if let Some(graph) = working_graph.as_ref() {
                            let pack = pack_from_working_graph(&knowledge_dir, &selectors, graph);
                            return serde_json::to_value(pack).map_err(|serialize| {
                                OrbitError::Execution(format!(
                                    "failed to serialize knowledge pack: {serialize}"
                                ))
                            });
                        }
                        return serde_json::to_value(error).map_err(|serialize| {
                            OrbitError::Execution(format!(
                                "failed to serialize knowledge error: {serialize}"
                            ))
                        });
                    }
                }
            }
        };
        let pack = if let Some(graph) = working_graph.as_ref() {
            overlay_pack_with_working_graph(pack, &selectors, graph)
        } else {
            pack
        };

        serde_json::to_value(pack)
            .map_err(|error| OrbitError::Execution(format!("serialize knowledge pack: {error}")))
    }
}

fn parse_selector_strings(input: &Value) -> Result<Vec<String>, OrbitError> {
    let raw = input
        .get("selectors")
        .ok_or_else(|| OrbitError::InvalidInput("missing `selectors`".to_string()))?;
    let items = raw
        .as_array()
        .ok_or_else(|| OrbitError::InvalidInput("`selectors` must be an array".to_string()))?;
    if items.is_empty() {
        return Err(OrbitError::InvalidInput(
            "`selectors` must contain at least one selector".to_string(),
        ));
    }

    items
        .iter()
        .map(|item| {
            item.as_str().map(str::to_string).ok_or_else(|| {
                OrbitError::InvalidInput("`selectors` entries must be strings".to_string())
            })
        })
        .collect()
}

fn resolve_knowledge_dir(ctx: &ToolContext, input: &Value) -> Result<PathBuf, OrbitError> {
    super::knowledge_write::resolve_knowledge_dir(ctx, input)
}
