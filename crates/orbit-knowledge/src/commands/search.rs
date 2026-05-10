use regex::Regex;

use crate::commands::GraphCommandContext;
use crate::graph::navigator::GraphNodeRef;
use crate::graph::{GraphIndexSearchRow, GraphReadOptions};
use crate::service::{GraphContextService, MatchedLine, SearchHit};
use crate::{KnowledgeError, graph::nodes::CodebaseGraphV1};

const DEFAULT_RANKING_HEADROOM: usize = 10;
const DEFAULT_RANKING_HARD_CAP: usize = 5_000;
pub const SOURCE_REGEX_UNBOUNDED_LIMIT_MAX: usize = 200;

#[derive(Debug, Clone)]
pub struct SearchInput {
    pub context: GraphCommandContext,
    pub query: String,
    pub node_type: Option<String>,
    pub kind_filter: Option<String>,
    pub prefix: Option<String>,
    pub source_regex: Option<Regex>,
    pub include_non_code: bool,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct DefaultSearchInput<'a> {
    pub graph: &'a CodebaseGraphV1,
    pub query: &'a str,
    pub limit: usize,
    pub include_non_code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub total: usize,
    pub hits: Vec<SearchResultItem>,
    pub used_index: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResultItem {
    pub selector: String,
    pub name: String,
    pub kind: String,
    pub file: Option<String>,
    pub matched_lines: Vec<MatchedLine>,
}

pub fn run(input: SearchInput) -> Result<SearchResult, KnowledgeError> {
    let has_source_regex = input.source_regex.is_some();
    let use_default_ranking = input.node_type.is_none()
        && input.kind_filter.is_none()
        && input.prefix.is_none()
        && !has_source_regex;

    if use_default_ranking
        && let Some(result) = try_default_search_via_sql_index(
            &input.context,
            &input.query,
            input.include_non_code,
            input.limit,
        )?
    {
        return Ok(result);
    }

    let graph = input.context.read_graph(GraphReadOptions {
        hydrate_file_source: has_source_regex,
        hydrate_leaf_source: has_source_regex,
    })?;
    let svc = GraphContextService::new(&graph);

    let type_strs: Vec<&str> = input.node_type.iter().map(String::as_str).collect();
    let node_types = if type_strs.is_empty() {
        None
    } else {
        Some(type_strs.as_slice())
    };
    let search_limit = if use_default_ranking {
        default_ranking_search_limit(input.limit)
    } else {
        input.limit
    };
    let candidate_scan_limit =
        if has_source_regex && input.prefix.is_none() && input.query.trim().is_empty() {
            Some(SOURCE_REGEX_UNBOUNDED_LIMIT_MAX)
        } else {
            None
        };
    let (service_total, hits) = svc
        .search_hits_with_total_bounded(
            &input.query,
            node_types,
            input.prefix.as_deref(),
            input.kind_filter.as_deref(),
            input.source_regex.as_ref(),
            search_limit,
            candidate_scan_limit,
        )
        .map_err(|error| {
            KnowledgeError::invalid_data(format!(
                "`source_regex` scanned more than {} source candidates; provide `prefix` or non-empty `query` to narrow the search",
                error.limit
            ))
        })?;

    if use_default_ranking {
        let nodes = hits.into_iter().map(|hit| hit.node).collect();
        let ranked = rank_default_search_results(nodes, input.include_non_code);
        let total = ranked.len();
        let hits = ranked
            .into_iter()
            .take(input.limit)
            .map(|node| search_item_for_node(&svc, node, Vec::new()))
            .collect();
        Ok(SearchResult {
            total,
            hits,
            used_index: false,
        })
    } else {
        Ok(SearchResult {
            total: service_total,
            hits: hits
                .into_iter()
                .map(|hit| search_item_for_hit(&svc, hit))
                .collect(),
            used_index: false,
        })
    }
}

pub fn default_search(input: DefaultSearchInput<'_>) -> Result<SearchResult, KnowledgeError> {
    let svc = GraphContextService::new(input.graph);
    let search_limit = default_ranking_search_limit(input.limit);
    let (_total, hits) =
        svc.search_hits_with_total(input.query, None, None, None, None, search_limit);
    let nodes = hits.into_iter().map(|hit| hit.node).collect();
    let ranked = rank_default_search_results(nodes, input.include_non_code);
    let total = ranked.len();
    let hits = ranked
        .into_iter()
        .take(input.limit)
        .map(|node| search_item_for_node(&svc, node, Vec::new()))
        .collect();
    Ok(SearchResult {
        total,
        hits,
        used_index: false,
    })
}

fn try_default_search_via_sql_index(
    context: &GraphCommandContext,
    query: &str,
    include_non_code: bool,
    limit: usize,
) -> Result<Option<SearchResult>, KnowledgeError> {
    let Some(reader) = context.open_current_graph_index()? else {
        return Ok(None);
    };

    let query_lower = query.trim().to_lowercase();
    let scan_cap = default_ranking_search_limit(limit);
    let rows = reader
        .search_substring(&query_lower, scan_cap)
        .map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!(
                "query graph sqlite substring search: {error}"
            ))
        })?;

    let ranked = rank_sql_default_search_results(rows, include_non_code);
    let total = ranked.len();
    let hits = ranked
        .into_iter()
        .take(limit)
        .map(search_item_for_row)
        .collect();
    Ok(Some(SearchResult {
        total,
        hits,
        used_index: true,
    }))
}

fn search_item_for_hit(svc: &GraphContextService<'_>, hit: SearchHit<'_>) -> SearchResultItem {
    search_item_for_node(svc, hit.node, hit.matched_lines)
}

fn search_item_for_node(
    svc: &GraphContextService<'_>,
    node: GraphNodeRef<'_>,
    matched_lines: Vec<MatchedLine>,
) -> SearchResultItem {
    let kind = match node {
        GraphNodeRef::Dir(_) => "dir".to_string(),
        GraphNodeRef::File(_) => "file".to_string(),
        GraphNodeRef::Leaf(leaf) => leaf.kind.to_string(),
    };
    let file = match node {
        GraphNodeRef::Leaf(leaf) => leaf
            .base
            .location
            .split_once('#')
            .map(|(path, _)| path.to_string()),
        GraphNodeRef::File(file) => Some(file.base.location.clone()),
        GraphNodeRef::Dir(_) => None,
    };

    SearchResultItem {
        selector: svc.selector_for_node(node),
        name: node.base().name.clone(),
        kind,
        file,
        matched_lines,
    }
}

fn search_item_for_row(row: GraphIndexSearchRow) -> SearchResultItem {
    let selector = selector_for_search_row(&row);
    let kind = kind_for_search_row(&row);
    let file = file_for_search_row(&row);
    SearchResultItem {
        selector,
        name: row.name,
        kind,
        file,
        matched_lines: Vec::new(),
    }
}

fn rank_sql_default_search_results(
    rows: Vec<GraphIndexSearchRow>,
    include_non_code: bool,
) -> Vec<GraphIndexSearchRow> {
    let mut ranked: Vec<(usize, usize, GraphIndexSearchRow)> = rows
        .into_iter()
        .enumerate()
        .filter_map(|(index, row)| {
            let rank = default_search_rank_for_row(&row);
            if !include_non_code && rank == 2 {
                return None;
            }
            Some((rank, index, row))
        })
        .collect();

    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    ranked.into_iter().map(|(_, _, row)| row).collect()
}

fn default_search_rank_for_row(row: &GraphIndexSearchRow) -> usize {
    match row.node_type.as_str() {
        "leaf" if row.kind.as_deref().is_some_and(is_code_symbol_kind) => 0,
        "leaf" => 2,
        "file" | "dir" => path_search_rank(&row.location),
        _ => 2,
    }
}

fn selector_for_search_row(row: &GraphIndexSearchRow) -> String {
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

fn kind_for_search_row(row: &GraphIndexSearchRow) -> String {
    match row.node_type.as_str() {
        "leaf" => row.kind.clone().unwrap_or_else(|| "symbol".to_string()),
        other => other.to_string(),
    }
}

fn file_for_search_row(row: &GraphIndexSearchRow) -> Option<String> {
    match row.node_type.as_str() {
        "leaf" => row
            .location
            .split_once('#')
            .map(|(path, _)| path.to_string()),
        "file" => Some(row.location.clone()),
        _ => None,
    }
}

fn default_ranking_search_limit(limit: usize) -> usize {
    limit
        .saturating_mul(DEFAULT_RANKING_HEADROOM)
        .min(DEFAULT_RANKING_HARD_CAP)
}

fn rank_default_search_results<'a>(
    nodes: Vec<GraphNodeRef<'a>>,
    include_non_code: bool,
) -> Vec<GraphNodeRef<'a>> {
    let mut ranked: Vec<(usize, usize, GraphNodeRef<'a>)> = nodes
        .into_iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let rank = default_search_rank(node);
            if !include_non_code && rank == 2 {
                return None;
            }
            Some((rank, index, node))
        })
        .collect();

    ranked.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    ranked.into_iter().map(|(_, _, node)| node).collect()
}

fn default_search_rank(node: GraphNodeRef<'_>) -> usize {
    match node {
        GraphNodeRef::Leaf(leaf) if is_code_symbol_kind(leaf.kind.to_string().as_str()) => 0,
        GraphNodeRef::Leaf(_) => 2,
        GraphNodeRef::File(file) => path_search_rank(&file.base.location),
        GraphNodeRef::Dir(dir) => path_search_rank(&dir.base.location),
    }
}

fn path_search_rank(path: &str) -> usize {
    if is_non_code_path(path) { 2 } else { 1 }
}

fn is_code_symbol_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function"
            | "method"
            | "struct"
            | "trait"
            | "impl"
            | "class"
            | "interface"
            | "field"
            | "module"
    )
}

fn is_non_code_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    matches!(
        extension.as_str(),
        "md" | "txt" | "rst" | "adoc" | "yaml" | "yml" | "toml" | "json" | "jsonc" | "csv" | "tsv"
    ) || lower.starts_with("docs/")
}

#[cfg(test)]
mod tests {
    use crate::graph::GraphIndexReader;
    use crate::graph::nodes::{
        BaseNodeFields, CodebaseGraphV1, DirNode, FileNode, LeafKind, LeafNode,
    };
    use crate::graph::object_store::GraphObjectStore;
    use crate::service::GraphContextService;

    use super::*;

    #[test]
    fn default_search_ranking_matches_snapshot() {
        let graph = graph_for_default_ranking_snapshot();
        let result = default_search(DefaultSearchInput {
            graph: &graph,
            query: "fixture",
            limit: 10,
            include_non_code: true,
        })
        .expect("default search");

        let snapshot: Vec<_> = result
            .hits
            .iter()
            .map(|hit| {
                (
                    hit.selector.as_str(),
                    hit.kind.as_str(),
                    hit.file.as_deref().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(
            snapshot,
            vec![
                (
                    "symbol:src/fixture.rs#fixture_fn:function",
                    "function",
                    "src/fixture.rs"
                ),
                ("file:src/fixture.rs", "file", "src/fixture.rs"),
                (
                    "symbol:README.md#fixture_doc:section",
                    "section",
                    "README.md"
                ),
            ]
        );
    }

    #[test]
    fn default_ranking_search_uses_finite_capped_bound_for_large_graph() {
        const REQUESTED_LIMIT: usize = 10;
        const FIXTURE_LEAF_COUNT: usize = 1_000;

        let graph = graph_with_matching_leaves(FIXTURE_LEAF_COUNT);
        let service = GraphContextService::new(&graph);
        let search_limit = default_ranking_search_limit(REQUESTED_LIMIT);

        assert!(search_limit <= DEFAULT_RANKING_HARD_CAP);
        assert_eq!(search_limit, REQUESTED_LIMIT * DEFAULT_RANKING_HEADROOM);

        let (_total, hits) =
            service.search_hits_with_total("fixture", None, None, None, None, search_limit);

        assert_eq!(hits.len(), search_limit);
    }

    #[test]
    fn default_ranking_search_limit_saturates_at_hard_cap() {
        let over_cap_limit = DEFAULT_RANKING_HARD_CAP / DEFAULT_RANKING_HEADROOM + 1;

        assert_eq!(
            default_ranking_search_limit(over_cap_limit),
            DEFAULT_RANKING_HARD_CAP
        );
    }

    #[test]
    fn sql_substring_path_matches_fallback_for_diverse_query_shapes() {
        let graph = graph_for_sql_search_tests();
        let (_temp_dir, reader) = index_reader_for_graph(&graph);

        for query in [
            "UniqueSymbol",
            "nique",
            "Unique Symbol",
            "^UniqueSymbol$",
            "src/core/",
            "src/special%_dir/",
        ] {
            let rows = sql_substring_selectors(&reader, query, false, 20);
            assert_eq!(
                rows,
                fallback_selectors(&graph, query, false, 20),
                "sql/fallback divergence for `{query}`"
            );
        }
    }

    #[test]
    fn sql_substring_path_honors_user_limit() {
        let graph = graph_with_repeated_name_leaves(25, "CommonSymbol");
        let (_temp_dir, reader) = index_reader_for_graph(&graph);

        let rows = sql_substring_selectors(&reader, "CommonSymbol", false, 10);
        assert_eq!(rows.len(), 10);
        assert_eq!(rows, fallback_selectors(&graph, "CommonSymbol", false, 10));
    }

    #[test]
    fn missing_or_stale_index_yields_no_sql_outcome() {
        let graph = graph_for_sql_search_tests();
        let (temp_dir, _reader) = index_reader_for_graph(&graph);
        let store = GraphObjectStore::new(temp_dir.path().join("graph"));

        assert!(
            GraphIndexReader::open_current(store.graph_sqlite_index_path(), "stale-root")
                .expect("open stale index")
                .is_none()
        );
        std::fs::remove_file(store.graph_sqlite_index_path()).expect("delete sqlite index");
        assert!(
            GraphIndexReader::open_current(store.graph_sqlite_index_path(), "missing-root")
                .expect("open missing index")
                .is_none()
        );
    }

    fn sql_substring_selectors(
        reader: &GraphIndexReader,
        query: &str,
        include_non_code: bool,
        limit: usize,
    ) -> Vec<String> {
        let scan_cap = default_ranking_search_limit(limit);
        let rows = reader
            .search_substring(&query.trim().to_lowercase(), scan_cap)
            .expect("sql substring search");
        let ranked = rank_sql_default_search_results(rows, include_non_code);
        ranked
            .into_iter()
            .take(limit)
            .map(|row| selector_for_search_row(&row))
            .collect()
    }

    fn fallback_selectors(
        graph: &CodebaseGraphV1,
        query: &str,
        include_non_code: bool,
        limit: usize,
    ) -> Vec<String> {
        default_search(DefaultSearchInput {
            graph,
            query,
            include_non_code,
            limit,
        })
        .expect("default search")
        .hits
        .into_iter()
        .map(|hit| hit.selector)
        .collect()
    }

    fn graph_for_default_ranking_snapshot() -> CodebaseGraphV1 {
        let root_id = "dir:.".to_string();
        let code_file_id = "file:src/fixture.rs".to_string();
        let doc_file_id = "file:README.md".to_string();
        let code_leaf_id = "symbol:src/fixture.rs#fixture_fn:function".to_string();
        let doc_leaf_id = "symbol:README.md#fixture_doc:section".to_string();

        CodebaseGraphV1 {
            root_dir_id: root_id.clone(),
            dirs: vec![DirNode {
                base: base_node(&root_id, ".", ".", "", None),
                dir_children: Vec::new(),
                file_children: vec![code_file_id.clone(), doc_file_id.clone()],
            }],
            files: vec![
                FileNode {
                    base: base_node(
                        &code_file_id,
                        "fixture.rs",
                        "src/fixture.rs",
                        "rust",
                        Some(&root_id),
                    ),
                    extension: Some("rs".to_string()),
                    source_blob_hash: None,
                    source: String::new(),
                    imports: Vec::new(),
                    exports: Vec::new(),
                    re_exports: Vec::new(),
                    leaf_children: vec![code_leaf_id.clone()],
                },
                FileNode {
                    base: base_node(
                        &doc_file_id,
                        "README.md",
                        "README.md",
                        "markdown",
                        Some(&root_id),
                    ),
                    extension: Some("md".to_string()),
                    source_blob_hash: None,
                    source: String::new(),
                    imports: Vec::new(),
                    exports: Vec::new(),
                    re_exports: Vec::new(),
                    leaf_children: vec![doc_leaf_id.clone()],
                },
            ],
            leaves: vec![
                LeafNode {
                    base: base_node(
                        &code_leaf_id,
                        "fixture_fn",
                        "src/fixture.rs#fixture_fn",
                        "rust",
                        Some(&code_file_id),
                    ),
                    kind: LeafKind::Function,
                    source: String::new(),
                    source_blob_hash: None,
                    source_hash: None,
                    file_hash_at_capture: None,
                    history: Vec::new(),
                    input_signature: Vec::new(),
                    output_signature: Vec::new(),
                    start_line: Some(1),
                    end_line: Some(1),
                    children: Vec::new(),
                },
                LeafNode {
                    base: base_node(
                        &doc_leaf_id,
                        "fixture_doc",
                        "README.md#fixture_doc",
                        "markdown",
                        Some(&doc_file_id),
                    ),
                    kind: LeafKind::Section { depth: 1 },
                    source: String::new(),
                    source_blob_hash: None,
                    source_hash: None,
                    file_hash_at_capture: None,
                    history: Vec::new(),
                    input_signature: Vec::new(),
                    output_signature: Vec::new(),
                    start_line: Some(1),
                    end_line: Some(1),
                    children: Vec::new(),
                },
            ],
        }
    }

    fn graph_with_matching_leaves(leaf_count: usize) -> CodebaseGraphV1 {
        let root_id = "dir:.".to_string();
        let file_id = "file:src/fixture.rs".to_string();
        let mut leaf_ids = Vec::with_capacity(leaf_count);
        let mut leaves = Vec::with_capacity(leaf_count);

        for index in 0..leaf_count {
            let name = format!("fixture_{index}");
            let leaf_id = format!("symbol:src/fixture.rs#{name}:function");
            leaf_ids.push(leaf_id.clone());
            leaves.push(leaf_node(
                &leaf_id,
                &name,
                &format!("src/fixture.rs#{name}"),
                Some(&file_id),
                (index + 1) as u32,
            ));
        }

        CodebaseGraphV1 {
            root_dir_id: root_id.clone(),
            dirs: vec![DirNode {
                base: base_node(&root_id, ".", ".", "", None),
                dir_children: Vec::new(),
                file_children: vec![file_id.clone()],
            }],
            files: vec![FileNode {
                base: base_node(
                    &file_id,
                    "fixture.rs",
                    "src/fixture.rs",
                    "rust",
                    Some(&root_id),
                ),
                extension: Some("rs".to_string()),
                source_blob_hash: None,
                source: String::new(),
                imports: Vec::new(),
                exports: Vec::new(),
                re_exports: Vec::new(),
                leaf_children: leaf_ids,
            }],
            leaves,
        }
    }

    fn graph_for_sql_search_tests() -> CodebaseGraphV1 {
        let root_id = "dir:.".to_string();
        let src_dir_id = "dir:src".to_string();
        let core_dir_id = "dir:src/core".to_string();
        let special_dir_id = "dir:src/special%_dir".to_string();
        let lookalike_dir_id = "dir:src/specialABdir".to_string();
        let core_file_id = "file:src/core/main.rs".to_string();
        let special_file_id = "file:src/special%_dir/mod.rs".to_string();
        let lookalike_file_id = "file:src/specialABdir/mod.rs".to_string();
        let unique_leaf_id = "symbol:src/core/main.rs#UniqueSymbol:function".to_string();
        let escaped_leaf_id = "symbol:src/special%_dir/mod.rs#EscapedSymbol:function".to_string();
        let lookalike_leaf_id =
            "symbol:src/specialABdir/mod.rs#LookalikeSymbol:function".to_string();

        CodebaseGraphV1 {
            root_dir_id: root_id.clone(),
            dirs: vec![
                DirNode {
                    base: base_node(&root_id, ".", ".", "", None),
                    dir_children: vec![src_dir_id.clone()],
                    file_children: Vec::new(),
                },
                DirNode {
                    base: base_node(&src_dir_id, "src", "src/", "", Some(&root_id)),
                    dir_children: vec![
                        core_dir_id.clone(),
                        special_dir_id.clone(),
                        lookalike_dir_id.clone(),
                    ],
                    file_children: Vec::new(),
                },
                DirNode {
                    base: base_node(&core_dir_id, "core", "src/core/", "", Some(&src_dir_id)),
                    dir_children: Vec::new(),
                    file_children: vec![core_file_id.clone()],
                },
                DirNode {
                    base: base_node(
                        &special_dir_id,
                        "special%_dir",
                        "src/special%_dir/",
                        "",
                        Some(&src_dir_id),
                    ),
                    dir_children: Vec::new(),
                    file_children: vec![special_file_id.clone()],
                },
                DirNode {
                    base: base_node(
                        &lookalike_dir_id,
                        "specialABdir",
                        "src/specialABdir/",
                        "",
                        Some(&src_dir_id),
                    ),
                    dir_children: Vec::new(),
                    file_children: vec![lookalike_file_id.clone()],
                },
            ],
            files: vec![
                file_node(
                    &core_file_id,
                    "main.rs",
                    "src/core/main.rs",
                    Some(&core_dir_id),
                    vec![unique_leaf_id.clone()],
                ),
                file_node(
                    &special_file_id,
                    "mod.rs",
                    "src/special%_dir/mod.rs",
                    Some(&special_dir_id),
                    vec![escaped_leaf_id.clone()],
                ),
                file_node(
                    &lookalike_file_id,
                    "mod.rs",
                    "src/specialABdir/mod.rs",
                    Some(&lookalike_dir_id),
                    vec![lookalike_leaf_id.clone()],
                ),
            ],
            leaves: vec![
                leaf_node(
                    &unique_leaf_id,
                    "UniqueSymbol",
                    "src/core/main.rs#UniqueSymbol",
                    Some(&core_file_id),
                    1,
                ),
                leaf_node(
                    &escaped_leaf_id,
                    "EscapedSymbol",
                    "src/special%_dir/mod.rs#EscapedSymbol",
                    Some(&special_file_id),
                    2,
                ),
                leaf_node(
                    &lookalike_leaf_id,
                    "LookalikeSymbol",
                    "src/specialABdir/mod.rs#LookalikeSymbol",
                    Some(&lookalike_file_id),
                    3,
                ),
            ],
        }
    }

    fn graph_with_repeated_name_leaves(leaf_count: usize, name: &str) -> CodebaseGraphV1 {
        let root_id = "dir:.".to_string();
        let dir_id = "dir:src/limit".to_string();
        let mut file_ids = Vec::with_capacity(leaf_count);
        let mut files = Vec::with_capacity(leaf_count);
        let mut leaves = Vec::with_capacity(leaf_count);

        for index in 0..leaf_count {
            let file_id = format!("file:src/limit/file_{index:05}.rs");
            let location = format!("src/limit/file_{index:05}.rs");
            let leaf_id = format!("symbol:{location}#{name}:function");
            file_ids.push(file_id.clone());
            files.push(file_node(
                &file_id,
                &format!("file_{index:05}.rs"),
                &location,
                Some(&dir_id),
                vec![leaf_id.clone()],
            ));
            leaves.push(leaf_node(
                &leaf_id,
                name,
                &format!("{location}#{name}"),
                Some(&file_id),
                (index + 1) as u32,
            ));
        }

        CodebaseGraphV1 {
            root_dir_id: root_id.clone(),
            dirs: vec![
                DirNode {
                    base: base_node(&root_id, ".", ".", "", None),
                    dir_children: vec![dir_id.clone()],
                    file_children: Vec::new(),
                },
                DirNode {
                    base: base_node(&dir_id, "limit", "src/limit/", "", Some(&root_id)),
                    dir_children: Vec::new(),
                    file_children: file_ids,
                },
            ],
            files,
            leaves,
        }
    }

    fn index_reader_for_graph(graph: &CodebaseGraphV1) -> (tempfile::TempDir, GraphIndexReader) {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let store = GraphObjectStore::new(temp_dir.path().join("graph"));
        let current_ref = store.write_graph(graph).expect("write graph");
        let reader = GraphIndexReader::open_current(
            store.graph_sqlite_index_path(),
            &current_ref.root_graph_hash,
        )
        .expect("open sqlite index")
        .expect("current sqlite index");
        (temp_dir, reader)
    }

    fn file_node(
        id: &str,
        name: &str,
        location: &str,
        parent_id: Option<&str>,
        leaf_children: Vec<String>,
    ) -> FileNode {
        FileNode {
            base: base_node(id, name, location, "rust", parent_id),
            extension: Some("rs".to_string()),
            source_blob_hash: None,
            source: String::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            re_exports: Vec::new(),
            leaf_children,
        }
    }

    fn leaf_node(
        id: &str,
        name: &str,
        location: &str,
        parent_id: Option<&str>,
        line: u32,
    ) -> LeafNode {
        LeafNode {
            base: base_node(id, name, location, "rust", parent_id),
            kind: LeafKind::Function,
            source: String::new(),
            source_blob_hash: None,
            source_hash: None,
            file_hash_at_capture: None,
            history: Vec::new(),
            input_signature: Vec::new(),
            output_signature: Vec::new(),
            start_line: Some(line),
            end_line: Some(line),
            children: Vec::new(),
        }
    }

    fn base_node(
        id: &str,
        name: &str,
        location: &str,
        language: &str,
        parent_id: Option<&str>,
    ) -> BaseNodeFields {
        BaseNodeFields {
            id: id.to_string(),
            identity_key: id.to_string(),
            object_hash: None,
            name: name.to_string(),
            location: location.to_string(),
            language: language.to_string(),
            description: String::new(),
            parent_id: parent_id.map(str::to_string),
            is_locked: false,
            lineage_locked: false,
            lock_owner: None,
            lock_reason: String::new(),
        }
    }
}
