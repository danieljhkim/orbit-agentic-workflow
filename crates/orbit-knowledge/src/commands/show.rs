use serde_json::Value;

use crate::commands::GraphCommandContext;
use crate::graph::object_store::{GraphObjectStore, resolve_graph_read_target};
use crate::graph::{GraphIndexNodeRow, GraphNode, GraphReadOptions};
use crate::service::{GraphContextService, NodeContext};
use crate::{KnowledgeError, Selector};

#[derive(Debug, Clone)]
pub struct ShowInput {
    pub context: GraphCommandContext,
    pub selector: String,
    pub depth: usize,
    pub max_siblings: usize,
    pub max_children: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShowResult {
    pub selector: String,
    pub lineage: Vec<String>,
    pub siblings: Vec<String>,
    pub children: Vec<String>,
    pub details: ShowNodeDetails,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShowNodeDetails {
    Dir,
    File {
        source: Option<String>,
        source_blob_hash: Option<String>,
        imports: Vec<String>,
        exports: Vec<String>,
        re_exports: Vec<Value>,
    },
    Leaf {
        source: String,
        start_line: Option<u32>,
        end_line: Option<u32>,
    },
}

pub fn run(input: ShowInput) -> Result<ShowResult, KnowledgeError> {
    let selector: Selector = input
        .selector
        .parse()
        .map_err(|error| KnowledgeError::invalid_data(format!("{error}")))?;

    if let Some(result) = try_show_via_sql_index(
        &input.context,
        &selector,
        input.depth,
        input.max_siblings,
        input.max_children,
    )? {
        return Ok(result);
    }

    let graph = input.context.read_graph(GraphReadOptions {
        hydrate_file_source: true,
        hydrate_leaf_source: true,
    })?;
    let svc = GraphContextService::new(&graph);
    let node = svc
        .resolve_selector(&selector)
        .map_err(|error| KnowledgeError::invalid_data(error.to_string()))?;
    let context = svc
        .bounded_context(
            node.id(),
            input.depth,
            input.max_siblings,
            input.max_children,
        )
        .map_err(|error| KnowledgeError::knowledge_unavailable(error.to_string()))?;

    Ok(result_from_context(&svc, &context))
}

fn try_show_via_sql_index(
    context: &GraphCommandContext,
    selector: &Selector,
    depth: usize,
    max_siblings: usize,
    max_children: usize,
) -> Result<Option<ShowResult>, KnowledgeError> {
    if context.explicit_ref.is_none()
        && !context.explicit_knowledge_dir
        && let Some(workspace_root) = context.workspace_root.as_deref()
    {
        let _ = crate::pipeline::ensure_fresh(&context.knowledge_dir, workspace_root);
    }

    let read_target = match resolve_graph_read_target(
        context.workspace_root.as_deref(),
        context.explicit_ref.as_deref(),
    ) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let graph_store = GraphObjectStore::new(context.knowledge_dir.join("graph"));
    if graph_store
        .prepare_refs_layout(read_target.default.as_ref())
        .is_err()
    {
        return Ok(None);
    }
    let resolved =
        match graph_store.resolve_ref(&read_target.requested, read_target.fallback.as_ref()) {
            Ok(resolved) => resolved,
            Err(_) => return Ok(None),
        };
    let reader = match crate::graph::GraphIndexReader::open_current(
        graph_store.graph_sqlite_index_path(),
        &resolved.current_ref.root_graph_hash,
    ) {
        Ok(Some(reader)) => reader,
        Ok(None) | Err(_) => return Ok(None),
    };

    let Some(row) = reader
        .find_node_by_selector(&selector.to_string())
        .map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!("query graph sqlite index: {error}"))
        })?
    else {
        return Ok(None);
    };

    let node = graph_store
        .read_node_by_object_hash(
            &row.id,
            &row.node_type,
            &row.object_hash,
            GraphReadOptions {
                hydrate_file_source: true,
                hydrate_leaf_source: true,
            },
        )
        .map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!("read graph node: {error}"))
        })?;
    let lineage = reader
        .lineage_for(row.parent_id.as_deref(), depth)
        .map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!("query graph sqlite lineage: {error}"))
        })?;
    let siblings = if let Some(parent_id) = row.parent_id.as_deref() {
        reader
            .children_of(parent_id, max_siblings.saturating_add(1))
            .map_err(|error| {
                KnowledgeError::knowledge_unavailable(format!(
                    "query graph sqlite siblings: {error}"
                ))
            })?
            .into_iter()
            .filter(|sibling| sibling.id != row.id)
            .take(max_siblings)
            .collect()
    } else {
        Vec::new()
    };
    let children = reader.children_of(&row.id, max_children).map_err(|error| {
        KnowledgeError::knowledge_unavailable(format!("query graph sqlite children: {error}"))
    })?;

    Ok(Some(result_from_sql_node(
        &node, &lineage, &siblings, &children,
    )))
}

fn result_from_context(service: &GraphContextService<'_>, context: &NodeContext<'_>) -> ShowResult {
    let node = context.node;
    ShowResult {
        selector: service.selector_for_node(node),
        lineage: context
            .lineage
            .iter()
            .map(|node| service.selector_for_node(*node))
            .collect(),
        siblings: context
            .siblings
            .iter()
            .map(|node| service.selector_for_node(*node))
            .collect(),
        children: context
            .children
            .iter()
            .map(|node| service.selector_for_node(*node))
            .collect(),
        details: match node {
            crate::graph::navigator::GraphNodeRef::Leaf(leaf) => ShowNodeDetails::Leaf {
                source: leaf.source.clone(),
                start_line: leaf.start_line,
                end_line: leaf.end_line,
            },
            crate::graph::navigator::GraphNodeRef::File(file) => ShowNodeDetails::File {
                source: (!file.source.is_empty()).then(|| file.source.clone()),
                source_blob_hash: file.source_blob_hash.clone(),
                imports: file.imports.clone(),
                exports: file.exports.clone(),
                re_exports: file
                    .re_exports
                    .iter()
                    .map(|value| serde_json::to_value(value).unwrap_or(Value::Null))
                    .collect(),
            },
            crate::graph::navigator::GraphNodeRef::Dir(_) => ShowNodeDetails::Dir,
        },
    }
}

fn result_from_sql_node(
    node: &GraphNode,
    lineage: &[GraphIndexNodeRow],
    siblings: &[GraphIndexNodeRow],
    children: &[GraphIndexNodeRow],
) -> ShowResult {
    ShowResult {
        selector: selector_for_graph_node(node),
        lineage: lineage.iter().map(selector_for_index_row).collect(),
        siblings: siblings.iter().map(selector_for_index_row).collect(),
        children: children.iter().map(selector_for_index_row).collect(),
        details: match node {
            GraphNode::Leaf(leaf) => ShowNodeDetails::Leaf {
                source: leaf.source.clone(),
                start_line: leaf.start_line,
                end_line: leaf.end_line,
            },
            GraphNode::File(file) => ShowNodeDetails::File {
                source: (!file.source.is_empty()).then(|| file.source.clone()),
                source_blob_hash: file.source_blob_hash.clone(),
                imports: file.imports.clone(),
                exports: file.exports.clone(),
                re_exports: file
                    .re_exports
                    .iter()
                    .map(|value| serde_json::to_value(value).unwrap_or(Value::Null))
                    .collect(),
            },
            GraphNode::Dir(_) => ShowNodeDetails::Dir,
        },
    }
}

fn selector_for_graph_node(node: &GraphNode) -> String {
    match node {
        GraphNode::Dir(dir) => {
            let path = dir.base.location.trim_end_matches('/');
            format!("dir:{path}")
        }
        GraphNode::File(file) => format!("file:{}", file.base.location),
        GraphNode::Leaf(leaf) => format!("symbol:{}:{}", leaf.base.location, leaf.kind),
    }
}

fn selector_for_index_row(row: &GraphIndexNodeRow) -> String {
    row.selector
        .clone()
        .unwrap_or_else(|| match row.node_type.as_str() {
            "dir" => {
                let path = row.location.trim_end_matches('/');
                format!("dir:{path}")
            }
            "file" => format!("file:{}", row.location),
            "leaf" => {
                let kind = row.kind.as_deref().unwrap_or_default();
                format!("symbol:{}:{kind}", row.location)
            }
            _ => row.id.clone(),
        })
}
