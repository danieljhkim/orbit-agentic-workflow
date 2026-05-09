use std::collections::{BTreeMap, HashMap};

use orbit_common::tracing::debug;
use orbit_common::types::{OrbitError, ToolParam, ToolSchema};
use orbit_knowledge::graph::GraphIndexReader;
use orbit_knowledge::graph::object_store::{GraphObjectStore, resolve_graph_read_target};
use orbit_knowledge::service::{
    GraphContextService, GraphOverview, GraphOverviewSummary, TopFileEntry, compact_from_overview,
};
use serde_json::{Value, json};

use crate::{Tool, ToolContext};

pub struct OrbitKnowledgeOverviewTool;

const AUTO_SUMMARY_FILE_THRESHOLD: usize = 20;
const FILE_THRESHOLD: usize = 50;
const SUMMARY_HINT: &str = "Use `prefix` to narrow the overview and get per-file symbol listings.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverviewFormat {
    Full,
    Summary,
}

impl OverviewFormat {
    fn parse(input: &Value) -> Result<Option<Self>, OrbitError> {
        let Some(format) = super::super::optional_string(input, "format")? else {
            return Ok(None);
        };

        match format.as_str() {
            "full" => Ok(Some(Self::Full)),
            "summary" => Ok(Some(Self::Summary)),
            other => Err(OrbitError::InvalidInput(format!(
                "`format` must be `full` or `summary`, got `{other}`"
            ))),
        }
    }

    fn default_for_scope(prefix: Option<&str>, file_count: usize) -> Self {
        if prefix.is_none() || file_count > AUTO_SUMMARY_FILE_THRESHOLD {
            Self::Summary
        } else {
            Self::Full
        }
    }
}

impl Tool for OrbitKnowledgeOverviewTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "orbit.graph.overview".to_string(),
            description: "Use when you need repo shape. Prefer over grep when you want counts, not text hits. Behavior: broad scopes default to `summary`; use `format:\"full\"` for per-file symbols.".to_string(),
            parameters: vec![
                ToolParam {
                    name: "prefix".to_string(),
                    description: "Only this path prefix.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "format".to_string(),
                    description: "`summary` or `full`.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                ToolParam {
                    name: "knowledge_dir".to_string(),
                    description: "Override knowledge dir.".to_string(),
                    param_type: "string".to_string(),
                    required: false,
                },
                super::super::graph_ref_param(),
            ],
            builtin: true,
        }
    }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let prefix = super::super::optional_string(&input, "prefix")?;
        let requested_format = OverviewFormat::parse(&input)?;
        let requested_format_label = requested_format_label(requested_format);

        if let Some(sql_summary) =
            try_summary_via_sql_index(ctx, &input, prefix.as_deref(), requested_format)?
        {
            return Ok(summary_response(
                sql_summary.summary,
                requested_format_label,
                sql_summary.downgraded,
            ));
        }

        let graph = super::load_graph_for_read(ctx, &input, Default::default())?;
        let svc = GraphContextService::new(&graph);
        let overview = svc.overview(prefix.as_deref());
        let resolved_format = requested_format.unwrap_or_else(|| {
            OverviewFormat::default_for_scope(prefix.as_deref(), overview.files.len())
        });
        let downgraded = matches!(requested_format, Some(OverviewFormat::Full))
            && overview.files.len() > FILE_THRESHOLD;
        let use_summary = matches!(resolved_format, OverviewFormat::Summary) || downgraded;
        Ok(if use_summary {
            summary_response(
                compact_from_overview(&overview, prefix.as_deref(), SUMMARY_HINT),
                requested_format_label,
                downgraded,
            )
        } else {
            full_response(overview, requested_format_label)
        })
    }
}

struct SqlOverviewSummary {
    summary: GraphOverviewSummary,
    downgraded: bool,
}

fn requested_format_label(requested_format: Option<OverviewFormat>) -> &'static str {
    match requested_format {
        Some(OverviewFormat::Full) => "full",
        Some(OverviewFormat::Summary) => "summary",
        None => "auto",
    }
}

fn try_summary_via_sql_index(
    ctx: &ToolContext,
    input: &Value,
    prefix: Option<&str>,
    requested_format: Option<OverviewFormat>,
) -> Result<Option<SqlOverviewSummary>, OrbitError> {
    if prefix.is_some() {
        debug!("falling back to graph overview scan because scoped SQL summaries are unsupported");
        return Ok(None);
    }

    let Some(reader) = open_current_sql_index(ctx, input)? else {
        return Ok(None);
    };

    let (total_dirs, total_files, total_symbols) = reader
        .overview_counts()
        .map_err(|error| OrbitError::Execution(format!("query graph sqlite overview: {error}")))?;
    let resolved_format =
        requested_format.unwrap_or_else(|| OverviewFormat::default_for_scope(None, total_files));
    let downgraded =
        matches!(requested_format, Some(OverviewFormat::Full)) && total_files > FILE_THRESHOLD;
    let use_summary = matches!(resolved_format, OverviewFormat::Summary) || downgraded;
    if !use_summary {
        debug!("falling back to graph overview scan because full output is required");
        return Ok(None);
    }

    let top_files = reader
        .overview_top_files(10)
        .map_err(|error| OrbitError::Execution(format!("query graph sqlite overview: {error}")))?
        .into_iter()
        .map(|(selector, name, symbol_count)| TopFileEntry {
            selector,
            name,
            symbol_count,
        })
        .collect();

    Ok(Some(SqlOverviewSummary {
        summary: GraphOverviewSummary {
            total_dirs,
            total_files,
            total_symbols,
            languages: reader.overview_language_counts().map_err(|error| {
                OrbitError::Execution(format!("query graph sqlite overview: {error}"))
            })?,
            symbol_kinds: reader.overview_symbol_kind_counts().map_err(|error| {
                OrbitError::Execution(format!("query graph sqlite overview: {error}"))
            })?,
            dir_file_counts: reader.overview_dir_file_counts().map_err(|error| {
                OrbitError::Execution(format!("query graph sqlite overview: {error}"))
            })?,
            top_files,
            hint: SUMMARY_HINT.to_string(),
        },
        downgraded,
    }))
}

fn open_current_sql_index(
    ctx: &ToolContext,
    input: &Value,
) -> Result<Option<GraphIndexReader>, OrbitError> {
    let knowledge_dir = super::write::resolve_knowledge_dir(ctx, input)?;
    let explicit_ref = super::super::optional_string(input, "ref")?;

    if explicit_ref.is_none()
        && !super::has_explicit_knowledge_dir(input)
        && let Some(workspace_root) = ctx.workspace_root.as_deref()
    {
        let _ = orbit_knowledge::pipeline::ensure_fresh(&knowledge_dir, workspace_root);
    }

    let read_target =
        match resolve_graph_read_target(ctx.workspace_root.as_deref(), explicit_ref.as_deref()) {
            Ok(target) => target,
            Err(error) => {
                debug!(%error, "falling back to graph overview scan; graph ref target unavailable");
                return Ok(None);
            }
        };
    let graph_store = GraphObjectStore::new(knowledge_dir.join("graph"));
    if let Err(error) = graph_store.prepare_refs_layout(read_target.default.as_ref()) {
        debug!(%error, "falling back to graph overview scan; graph refs unavailable");
        return Ok(None);
    }
    let resolved =
        match graph_store.resolve_ref(&read_target.requested, read_target.fallback.as_ref()) {
            Ok(resolved) => resolved,
            Err(error) => {
                debug!(%error, "falling back to graph overview scan; graph ref unavailable");
                return Ok(None);
            }
        };

    match GraphIndexReader::open_current(
        graph_store.graph_sqlite_index_path(),
        &resolved.current_ref.root_graph_hash,
    ) {
        Ok(Some(reader)) => Ok(Some(reader)),
        Ok(None) => {
            debug!("falling back to graph overview scan; sqlite graph index is missing or stale");
            Ok(None)
        }
        Err(error) => {
            debug!(%error, "falling back to graph overview scan; sqlite graph index could not be opened");
            Ok(None)
        }
    }
}

fn full_response(overview: GraphOverview, requested_format: &str) -> Value {
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
        "mode": "full",
        "requested_format": requested_format,
        "total_dirs": overview.total_dirs,
        "total_files": overview.total_files,
        "total_symbols": overview.total_symbols,
        "languages": overview.languages,
        "symbol_kinds": overview.symbol_kinds,
        "files": files,
    })
}

fn summary_response(
    summary: GraphOverviewSummary,
    requested_format: &str,
    downgraded: bool,
) -> Value {
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
        "requested_format": requested_format,
        "downgraded": downgraded,
        "total_dirs": summary.total_dirs,
        "total_files": summary.total_files,
        "total_symbols": summary.total_symbols,
        "languages": sorted_counts(summary.languages),
        "symbol_kinds": sorted_counts(summary.symbol_kinds),
        "dir_file_counts": summary.dir_file_counts,
        "top_files": top_files,
        "hint": summary.hint,
    })
}

fn sorted_counts(counts: HashMap<String, usize>) -> BTreeMap<String, usize> {
    counts.into_iter().collect()
}
