use orbit_knowledge::graph::object_store::GraphObjectStore;
use orbit_knowledge::service::GraphContextService;
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeOverviewTool;

impl Tool for OrbitKnowledgeOverviewTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.overview".to_string(),
            description: "Return an aggregate overview of the knowledge graph: node counts, languages, symbol kinds, and per-file symbol listings.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "prefix".to_string(),
                    description: "Location prefix to scope the overview (e.g. `crates/orbit-knowledge/src`)".to_string(),
                    param_type: "string".to_string(),
                    required: false,
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
        let prefix = super::optional_string(&input, "prefix")?;
        let knowledge_dir = super::knowledge_write::resolve_knowledge_dir(ctx, &input)?;
        let graph_dir = knowledge_dir.join("graph");
        let graph = GraphObjectStore::new(graph_dir)
            .read_graph()
            .map_err(|e| OrbitError::Execution(format!("failed to load knowledge graph: {e}")))?;

        let svc = GraphContextService::new(&graph);
        let overview = svc.overview(prefix.as_deref());

        let files: Vec<Value> = overview
            .files
            .into_iter()
            .map(|f| {
                let symbols: Vec<Value> = f
                    .symbols
                    .into_iter()
                    .map(|s| {
                        json!({
                            "name": s.name,
                            "kind": s.kind,
                            "selector": s.selector,
                        })
                    })
                    .collect();
                json!({
                    "selector": f.selector,
                    "name": f.name,
                    "symbol_count": f.symbol_count,
                    "symbols": symbols,
                })
            })
            .collect();

        Ok(json!({
            "total_dirs": overview.total_dirs,
            "total_files": overview.total_files,
            "total_symbols": overview.total_symbols,
            "languages": overview.languages,
            "symbol_kinds": overview.symbol_kinds,
            "files": files,
        }))
    }
}
