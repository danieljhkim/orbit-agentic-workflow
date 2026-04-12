use orbit_knowledge::graph::object_store::GraphObjectStore;
use orbit_knowledge::service::{
    GraphContextService, GraphOverview, GraphOverviewSummary, compact_from_overview,
};
use orbit_types::{OrbitError, ToolParam, ToolSchema};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeOverviewTool;

const FILE_THRESHOLD: usize = 50;
const SUMMARY_HINT: &str = "Use `prefix` to narrow the overview and get per-file symbol listings.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverviewFormat {
    Full,
    Summary,
}

impl OverviewFormat {
    fn parse(input: &Value) -> Result<Self, OrbitError> {
        let Some(format) = super::optional_string(input, "format")? else {
            return Ok(Self::Full);
        };

        match format.as_str() {
            "full" => Ok(Self::Full),
            "summary" => Ok(Self::Summary),
            other => Err(OrbitError::InvalidInput(format!(
                "`format` must be `full` or `summary`, got `{other}`"
            ))),
        }
    }
}

impl Tool for OrbitKnowledgeOverviewTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.overview".to_string(),
            description: "Return an aggregate overview of the knowledge graph. Broad queries auto-compact above 50 files, and `format: \"summary\"` always forces the compact form.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "prefix".to_string(),
                    description: "Location prefix to scope the overview (e.g. `crates/orbit-knowledge/src`)".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "format".to_string(),
                    description: "Output format: `full` (default) returns per-file symbol listings when the scope stays under 50 files; `summary` always returns the compact form.".to_string(),
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
        let format = OverviewFormat::parse(&input)?;
        let knowledge_dir = super::knowledge_write::resolve_knowledge_dir(ctx, &input)?;
        let graph_dir = knowledge_dir.join("graph");
        let graph = GraphObjectStore::new(graph_dir)
            .read_graph()
            .map_err(|e| OrbitError::Execution(format!("failed to load knowledge graph: {e}")))?;

        let svc = GraphContextService::new(&graph);
        let overview = svc.overview(prefix.as_deref());
        let use_summary =
            matches!(format, OverviewFormat::Summary) || overview.files.len() > FILE_THRESHOLD;

        Ok(if use_summary {
            summary_response(compact_from_overview(
                &overview,
                prefix.as_deref(),
                SUMMARY_HINT,
            ))
        } else {
            full_response(overview)
        })
    }
}

fn full_response(overview: GraphOverview) -> Value {
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

    json!({
        "total_dirs": overview.total_dirs,
        "total_files": overview.total_files,
        "total_symbols": overview.total_symbols,
        "languages": overview.languages,
        "symbol_kinds": overview.symbol_kinds,
        "files": files,
    })
}

fn summary_response(summary: GraphOverviewSummary) -> Value {
    let top_files: Vec<Value> = summary
        .top_files
        .into_iter()
        .map(|file| {
            json!({
                "selector": file.selector,
                "name": file.name,
                "symbol_count": file.symbol_count,
            })
        })
        .collect();

    json!({
        "mode": "summary",
        "total_dirs": summary.total_dirs,
        "total_files": summary.total_files,
        "total_symbols": summary.total_symbols,
        "languages": summary.languages,
        "symbol_kinds": summary.symbol_kinds,
        "dir_file_counts": summary.dir_file_counts,
        "top_files": top_files,
        "hint": summary.hint,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use orbit_knowledge::graph::nodes::{
        BaseNodeFields, CodebaseGraphV1, DirNode, FileNode, LeafKind, LeafNode,
    };
    use serde_json::{Value, json};

    use crate::{Tool, ToolContext};

    use super::{OrbitKnowledgeOverviewTool, OverviewFormat};

    fn make_base(id: &str, name: &str, location: &str, parent: Option<&str>) -> BaseNodeFields {
        BaseNodeFields {
            id: id.to_string(),
            identity_key: String::new(),
            object_hash: None,
            name: name.to_string(),
            location: location.to_string(),
            language: "rust".to_string(),
            description: String::new(),
            parent_id: parent.map(str::to_string),
            is_locked: false,
            lineage_locked: false,
            lock_owner: None,
            lock_reason: String::new(),
        }
    }

    fn generated_graph(layout: &[(&str, usize)]) -> CodebaseGraphV1 {
        let mut root = DirNode {
            base: make_base("d-root", ".", "./", None),
            dir_children: Vec::new(),
            file_children: Vec::new(),
        };
        let mut dirs = Vec::new();
        let mut files = Vec::new();
        let mut leaves = Vec::new();

        for (dir_name, file_count) in layout {
            let dir_id = format!("d-{}", dir_name.replace('/', "-"));
            root.dir_children.push(dir_id.clone());
            let mut dir = DirNode {
                base: make_base(&dir_id, dir_name, &format!("{dir_name}/"), Some("d-root")),
                dir_children: vec![],
                file_children: vec![],
            };

            for file_index in 0..*file_count {
                let file_id = format!("f-{dir_name}-{file_index}");
                let file_name = format!("file_{file_index:03}.rs");
                let file_path = format!("{dir_name}/{file_name}");
                let symbol_count = (file_index % 5) + 1;
                let mut leaf_children = Vec::new();

                for symbol_index in 0..symbol_count {
                    let leaf_id = format!("l-{dir_name}-{file_index}-{symbol_index}");
                    leaf_children.push(leaf_id.clone());
                    leaves.push(LeafNode {
                        base: make_base(
                            &leaf_id,
                            &format!("item_{file_index:03}_{symbol_index}"),
                            &format!("{file_path}#item_{file_index:03}_{symbol_index}"),
                            Some(&file_id),
                        ),
                        kind: if symbol_index % 2 == 0 {
                            LeafKind::Function
                        } else {
                            LeafKind::Struct
                        },
                        source: format!("// {file_path} symbol {symbol_index}"),
                        source_blob_hash: None,
                        source_hash: None,
                        file_hash_at_capture: None,
                        history: vec![],
                        input_signature: vec![],
                        output_signature: vec![],
                        start_line: Some((symbol_index + 1) as u32),
                        end_line: Some((symbol_index + 1) as u32),
                        children: vec![],
                    });
                }

                dir.file_children.push(file_id.clone());
                files.push(FileNode {
                    base: make_base(&file_id, &file_name, &file_path, Some(&dir_id)),
                    extension: Some("rs".to_string()),
                    source_blob_hash: None,
                    imports: vec![],
                    exports: vec![],
                    leaf_children,
                });
            }

            dirs.push(dir);
        }

        let mut all_dirs = vec![root];
        all_dirs.extend(dirs);
        CodebaseGraphV1 {
            root_dir_id: "d-root".to_string(),
            dirs: all_dirs,
            files,
            leaves,
        }
    }

    fn write_graph_fixture(graph: &CodebaseGraphV1) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let knowledge_dir = dir.path().join("knowledge");
        super::GraphObjectStore::new(knowledge_dir.join("graph"))
            .write_graph(graph)
            .expect("write graph");
        (dir, knowledge_dir)
    }

    fn run_tool(input: Value) -> Value {
        OrbitKnowledgeOverviewTool
            .execute(&ToolContext::default(), input)
            .expect("overview should succeed")
    }

    #[test]
    fn schema_exposes_optional_format_parameter() {
        let schema = OrbitKnowledgeOverviewTool.schema();
        let format_param = schema
            .parameters
            .iter()
            .find(|param| param.name == "format")
            .expect("format param");

        assert_eq!(format_param.param_type, "string");
        assert!(!format_param.required);
    }

    #[test]
    fn format_defaults_to_full() {
        assert_eq!(
            OverviewFormat::parse(&json!({})).expect("default format"),
            OverviewFormat::Full
        );
    }

    #[test]
    fn format_accepts_summary() {
        assert_eq!(
            OverviewFormat::parse(&json!({ "format": "summary" })).expect("summary format"),
            OverviewFormat::Summary
        );
    }

    #[test]
    fn format_rejects_unknown_value() {
        let err = OverviewFormat::parse(&json!({ "format": "compact" }))
            .expect_err("unknown format should fail");

        assert!(matches!(err, orbit_types::OrbitError::InvalidInput(_)));
        assert_eq!(
            err.to_string(),
            "invalid input: `format` must be `full` or `summary`, got `compact`"
        );
    }

    #[test]
    fn tool_returns_summary_for_large_overview() {
        let graph = generated_graph(&[("crates", 160), ("docs", 60)]);
        let (_dir, knowledge_dir) = write_graph_fixture(&graph);
        let result = run_tool(json!({
            "knowledge_dir": knowledge_dir.display().to_string()
        }));

        assert_eq!(result["mode"], json!("summary"));
        assert_eq!(
            result["dir_file_counts"],
            json!({ "crates": 160, "docs": 60 })
        );
        assert_eq!(
            result["top_files"]
                .as_array()
                .expect("top files array")
                .len(),
            10
        );
        assert!(
            result["hint"]
                .as_str()
                .expect("hint string")
                .contains("prefix")
        );
        assert!(result.get("files").is_none());
        assert!(
            result["symbol_kinds"]["function"]
                .as_u64()
                .expect("function count")
                > 0
        );
        assert!(serde_json::to_string(&result).expect("serialize").len() < 5_000);
    }

    #[test]
    fn tool_returns_summary_when_force_format_summary() {
        let graph = generated_graph(&[("crates", 3)]);
        let (_dir, knowledge_dir) = write_graph_fixture(&graph);
        let result = run_tool(json!({
            "format": "summary",
            "prefix": "crates/",
            "knowledge_dir": knowledge_dir.display().to_string()
        }));

        assert_eq!(result["mode"], json!("summary"));
        assert!(result.get("files").is_none());
        assert_eq!(
            result["top_files"]
                .as_array()
                .expect("top files array")
                .len(),
            3
        );
    }

    #[test]
    fn tool_returns_full_below_threshold() {
        let graph = generated_graph(&[("crates", 40), ("docs", 20)]);
        let (_dir, knowledge_dir) = write_graph_fixture(&graph);
        let result = run_tool(json!({
            "prefix": "crates/",
            "knowledge_dir": knowledge_dir.display().to_string()
        }));

        assert!(result.get("mode").is_none());
        assert_eq!(result["total_files"], json!(40));
        let files = result["files"].as_array().expect("files array");
        assert_eq!(files.len(), 40);
        assert!(files[0].get("symbols").and_then(Value::as_array).is_some());
    }
}
