//! Service layer: selector resolution, search, and context pack rendering.

pub mod lineage;

use std::collections::HashMap;

use crate::error::KnowledgeError;
use crate::graph::navigator::{GraphNavigator, GraphNodeRef};
use crate::graph::nodes::CodebaseGraphV1;
use crate::selector::Selector;

/// High-level query service over a [`CodebaseGraphV1`].
///
/// Wraps a [`GraphNavigator`] and adds selector resolution, search,
/// and context-building helpers.
pub struct GraphContextService<'a> {
    graph: &'a CodebaseGraphV1,
    nav: GraphNavigator<'a>,
    /// location key → node ID (for selector resolution).
    /// Dirs/files use location as-is; leaves use "location:kind" to
    /// disambiguate struct vs impl at the same qualified name.
    location_index: HashMap<String, &'a str>,
}

impl<'a> GraphContextService<'a> {
    pub fn new(graph: &'a CodebaseGraphV1) -> Self {
        let nav = GraphNavigator::new(graph);

        let mut location_index = HashMap::new();
        for dir in &graph.dirs {
            // Normalize dir locations by stripping trailing slash so that
            // `dir:src` resolves regardless of whether the stored location
            // is `"src"` or `"src/"`.
            let key = dir.base.location.trim_end_matches('/').to_string();
            location_index.insert(key, dir.base.id.as_str());
        }
        for file in &graph.files {
            location_index.insert(file.base.location.clone(), file.base.id.as_str());
        }
        for leaf in &graph.leaves {
            let key = format!("{}:{}", leaf.base.location, leaf.kind);
            location_index.insert(key, leaf.base.id.as_str());
        }

        Self {
            graph,
            nav,
            location_index,
        }
    }

    pub fn navigator(&self) -> &GraphNavigator<'a> {
        &self.nav
    }

    // -----------------------------------------------------------------
    // Selector resolution
    // -----------------------------------------------------------------

    /// Resolve a [`Selector`] to a graph node.
    pub fn resolve_selector(
        &self,
        selector: &Selector,
    ) -> Result<GraphNodeRef<'a>, KnowledgeError> {
        let key = match selector {
            Selector::Dir { path } => path.trim_end_matches('/').to_string(),
            Selector::File { path } => path.clone(),
            Selector::Symbol { path, symbol, kind } => format!("{path}#{symbol}:{kind}"),
        };

        let node_id = self.location_index.get(key.as_str()).ok_or_else(|| {
            KnowledgeError::invalid_data(format!(
                "selector `{selector}` does not resolve to a node"
            ))
        })?;

        self.nav.get_node(node_id)
    }

    /// Resolve multiple selectors, returning (resolved, unresolved) pairs.
    pub fn resolve_many(&self, selectors: &[Selector]) -> (Vec<GraphNodeRef<'a>>, Vec<String>) {
        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();
        for sel in selectors {
            match self.resolve_selector(sel) {
                Ok(node) => resolved.push(node),
                Err(_) => unresolved.push(sel.to_string()),
            }
        }
        (resolved, unresolved)
    }

    // -----------------------------------------------------------------
    // Search
    // -----------------------------------------------------------------

    /// Search nodes by name substring, optionally filtered by node type,
    /// location prefix, and leaf kind. An empty `query` matches all nodes
    /// (browse mode).
    pub fn search(
        &self,
        query: &str,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
        limit: usize,
    ) -> Vec<GraphNodeRef<'a>> {
        let query_lower = query.to_lowercase();
        let browse = query_lower.is_empty();
        let mut results = Vec::new();

        let matches = |node: GraphNodeRef<'a>, ntype: &str| -> bool {
            if let Some(types) = node_types
                && !types.contains(&ntype)
            {
                return false;
            }
            if let Some(prefix) = location_prefix
                && !node.location().starts_with(prefix)
            {
                return false;
            }
            // kind_filter only applies to leaf nodes; non-leaves are excluded
            // when a kind filter is active.
            if let Some(kf) = kind_filter {
                match node {
                    GraphNodeRef::Leaf(l) => {
                        if l.kind.to_string() != kf {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            browse
                || node.base().name.to_lowercase().contains(&query_lower)
                || node.location().to_lowercase().contains(&query_lower)
        };

        for dir in &self.graph.dirs {
            let node = GraphNodeRef::Dir(dir);
            if matches(node, "dir") {
                results.push(node);
                if results.len() >= limit {
                    return results;
                }
            }
        }
        for file in &self.graph.files {
            let node = GraphNodeRef::File(file);
            if matches(node, "file") {
                results.push(node);
                if results.len() >= limit {
                    return results;
                }
            }
        }
        for leaf in &self.graph.leaves {
            let node = GraphNodeRef::Leaf(leaf);
            if matches(node, "symbol") {
                results.push(node);
                if results.len() >= limit {
                    return results;
                }
            }
        }

        results
    }

    /// Search returning structured results with name, kind, and file info.
    pub fn search_structured(
        &self,
        query: &str,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
        limit: usize,
    ) -> Vec<SearchResult> {
        let nodes = self.search(query, node_types, location_prefix, kind_filter, limit);
        nodes
            .into_iter()
            .map(|node| {
                let selector = self.selector_for_node(node);
                let name = node.base().name.to_string();
                let kind = match node {
                    GraphNodeRef::Dir(_) => "dir".to_string(),
                    GraphNodeRef::File(_) => "file".to_string(),
                    GraphNodeRef::Leaf(l) => l.kind.to_string(),
                };
                let file = match node {
                    GraphNodeRef::Leaf(l) => {
                        l.base.location.split_once('#').map(|(f, _)| f.to_string())
                    }
                    GraphNodeRef::File(f) => Some(f.base.location.clone()),
                    GraphNodeRef::Dir(_) => None,
                };
                SearchResult {
                    selector,
                    name,
                    kind,
                    file,
                }
            })
            .collect()
    }

    // -----------------------------------------------------------------
    // Context helpers
    // -----------------------------------------------------------------

    /// Build a selector string for a graph node.
    pub fn selector_for_node(&self, node: GraphNodeRef<'_>) -> String {
        match node {
            GraphNodeRef::Dir(d) => {
                let path = d.base.location.trim_end_matches('/');
                format!("dir:{path}")
            }
            GraphNodeRef::File(f) => format!("file:{}", f.base.location),
            GraphNodeRef::Leaf(l) => {
                format!("symbol:{}:{}", l.base.location, l.kind)
            }
        }
    }

    /// Get bounded context around a node: lineage, siblings, children.
    pub fn bounded_context(
        &self,
        node_id: &str,
        depth: usize,
        max_siblings: usize,
        max_children: usize,
    ) -> Result<NodeContext<'a>, KnowledgeError> {
        let node = self.nav.get_node(node_id)?;

        let lineage = self.nav.get_lineage(node_id, false)?;
        let bounded_lineage: Vec<GraphNodeRef<'a>> = if lineage.len() > depth {
            lineage[lineage.len() - depth..].to_vec()
        } else {
            lineage
        };

        let siblings = self.nav.get_siblings(node_id)?;
        let bounded_siblings: Vec<GraphNodeRef<'a>> =
            siblings.into_iter().take(max_siblings).collect();

        let children = self.nav.get_children(node_id)?;
        let bounded_children: Vec<GraphNodeRef<'a>> =
            children.into_iter().take(max_children).collect();

        Ok(NodeContext {
            node,
            lineage: bounded_lineage,
            siblings: bounded_siblings,
            children: bounded_children,
        })
    }

    // -----------------------------------------------------------------
    // Overview
    // -----------------------------------------------------------------

    /// Build an aggregate overview of the graph, optionally scoped by location prefix.
    pub fn overview(&self, location_prefix: Option<&str>) -> GraphOverview {
        let in_scope =
            |loc: &str| -> bool { location_prefix.map(|p| loc.starts_with(p)).unwrap_or(true) };

        let mut total_dirs = 0usize;
        let mut total_files = 0usize;
        let mut total_symbols = 0usize;
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut symbol_kinds: HashMap<String, usize> = HashMap::new();
        let mut file_overviews: Vec<FileOverview> = Vec::new();

        for dir in &self.graph.dirs {
            if in_scope(&dir.base.location) {
                total_dirs += 1;
            }
        }

        // Index leaves by parent file
        let mut file_leaves: HashMap<&str, Vec<SymbolBrief>> = HashMap::new();
        for leaf in &self.graph.leaves {
            if !in_scope(&leaf.base.location) {
                continue;
            }
            total_symbols += 1;
            let kind_str = leaf.kind.to_string();
            *symbol_kinds.entry(kind_str.clone()).or_default() += 1;
            let file_path = leaf
                .base
                .location
                .split_once('#')
                .map(|(f, _)| f)
                .unwrap_or(&leaf.base.location);
            file_leaves.entry(file_path).or_default().push(SymbolBrief {
                name: leaf.base.name.clone(),
                kind: kind_str,
                selector: self.selector_for_node(GraphNodeRef::Leaf(leaf)),
            });
        }

        for file in &self.graph.files {
            if !in_scope(&file.base.location) {
                continue;
            }
            total_files += 1;
            if !file.base.language.is_empty() {
                *languages.entry(file.base.language.clone()).or_default() += 1;
            }
            let symbols = file_leaves
                .remove(file.base.location.as_str())
                .unwrap_or_default();
            file_overviews.push(FileOverview {
                selector: self.selector_for_node(GraphNodeRef::File(file)),
                name: file.base.name.clone(),
                symbol_count: symbols.len(),
                symbols,
            });
        }

        GraphOverview {
            total_dirs,
            total_files,
            total_symbols,
            languages,
            symbol_kinds,
            files: file_overviews,
        }
    }

    // -----------------------------------------------------------------
    // References
    // -----------------------------------------------------------------

    /// Find graph nodes whose source mentions `symbol_name`.
    pub fn find_references(
        &self,
        symbol_name: &str,
        exclude_file: Option<&str>,
    ) -> Vec<ReferenceHit> {
        let mut hits: Vec<ReferenceHit> = Vec::new();

        for leaf in &self.graph.leaves {
            // Skip the definition itself
            if leaf.base.name == symbol_name {
                continue;
            }
            // Skip leaves in the excluded file
            let file_path = leaf
                .base
                .location
                .split_once('#')
                .map(|(f, _)| f)
                .unwrap_or(&leaf.base.location);
            if exclude_file == Some(file_path) {
                continue;
            }
            if leaf.source.contains(symbol_name) {
                hits.push(ReferenceHit {
                    selector: self.selector_for_node(GraphNodeRef::Leaf(leaf)),
                    name: leaf.base.name.clone(),
                    file: file_path.to_string(),
                    kind: leaf.kind.to_string(),
                });
            }
        }

        hits
    }
}

/// A single structured search result.
pub struct SearchResult {
    pub selector: String,
    pub name: String,
    /// `"dir"`, `"file"`, or a leaf kind like `"function"`, `"struct"`.
    pub kind: String,
    /// The containing file path (populated for file and leaf nodes).
    pub file: Option<String>,
}

/// Aggregate overview of a (scoped) graph.
pub struct GraphOverview {
    pub total_dirs: usize,
    pub total_files: usize,
    pub total_symbols: usize,
    pub languages: HashMap<String, usize>,
    pub symbol_kinds: HashMap<String, usize>,
    pub files: Vec<FileOverview>,
}

pub struct FileOverview {
    pub selector: String,
    pub name: String,
    pub symbol_count: usize,
    pub symbols: Vec<SymbolBrief>,
}

pub struct SymbolBrief {
    pub name: String,
    pub kind: String,
    pub selector: String,
}

/// A reference hit from `find_references`.
pub struct ReferenceHit {
    pub selector: String,
    pub name: String,
    pub file: String,
    pub kind: String,
}

/// Bounded context around a single node.
pub struct NodeContext<'a> {
    pub node: GraphNodeRef<'a>,
    pub lineage: Vec<GraphNodeRef<'a>>,
    pub siblings: Vec<GraphNodeRef<'a>>,
    pub children: Vec<GraphNodeRef<'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::nodes::*;

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

    fn fixture_graph() -> CodebaseGraphV1 {
        CodebaseGraphV1 {
            root_dir_id: "d-root".to_string(),
            dirs: vec![
                DirNode {
                    base: make_base("d-root", ".", "./", None),
                    dir_children: vec!["d-src".to_string()],
                    file_children: vec![],
                },
                DirNode {
                    base: make_base("d-src", "src", "src/", Some("d-root")),
                    dir_children: vec![],
                    file_children: vec!["f-lib".to_string()],
                },
            ],
            files: vec![FileNode {
                base: make_base("f-lib", "lib.rs", "src/lib.rs", Some("d-src")),
                extension: Some("rs".to_string()),
                source_blob_hash: None,
                imports: vec![],
                exports: vec![],
                leaf_children: vec!["l-hello".to_string()],
            }],
            leaves: vec![LeafNode {
                base: make_base("l-hello", "hello", "src/lib.rs#hello", Some("f-lib")),
                kind: LeafKind::Function,
                source: "pub fn hello() {}".to_string(),
                source_blob_hash: None,
                source_hash: None,
                file_hash_at_capture: None,
                history: vec![],
                input_signature: vec![],
                output_signature: vec![],
                start_line: Some(1),
                end_line: Some(1),
                children: vec![],
            }],
        }
    }

    #[test]
    fn resolve_file_selector() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let sel: Selector = "file:src/lib.rs".parse().unwrap();
        let node = svc.resolve_selector(&sel).unwrap();
        assert_eq!(node.id(), "f-lib");
    }

    #[test]
    fn resolve_dir_selector() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let sel: Selector = "dir:src".parse().unwrap();
        let node = svc.resolve_selector(&sel).unwrap();
        assert_eq!(node.id(), "d-src");
    }

    #[test]
    fn resolve_leaf_selector() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let sel: Selector = "symbol:src/lib.rs#hello:function".parse().unwrap();
        let node = svc.resolve_selector(&sel).unwrap();
        assert_eq!(node.id(), "l-hello");
    }

    #[test]
    fn resolve_unknown_returns_error() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let sel: Selector = "file:nonexistent.rs".parse().unwrap();
        assert!(svc.resolve_selector(&sel).is_err());
    }

    #[test]
    fn search_by_name() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let results = svc.search("hello", None, None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), "l-hello");
    }

    #[test]
    fn search_with_type_filter() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let results = svc.search("lib", Some(&["file"]), None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), "f-lib");
    }

    #[test]
    fn search_with_location_prefix() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let results = svc.search("hello", None, Some("src/"), None, 10);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_empty_query_returns_all() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        // Empty query = browse mode, returns all nodes up to limit
        let results = svc.search("", None, None, None, 100);
        // fixture has 2 dirs + 1 file + 1 leaf = 4 nodes
        assert_eq!(results.len(), 4);
    }

    #[test]
    fn search_with_kind_filter() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let results = svc.search("", None, None, Some("function"), 100);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id(), "l-hello");
    }

    #[test]
    fn search_structured_returns_name_and_kind() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let results = svc.search_structured("hello", None, None, None, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "hello");
        assert_eq!(results[0].kind, "function");
        assert_eq!(results[0].file.as_deref(), Some("src/lib.rs"));
        assert_eq!(results[0].selector, "symbol:src/lib.rs#hello:function");
    }

    #[test]
    fn bounded_context_includes_lineage_and_children() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let ctx = svc.bounded_context("f-lib", 2, 5, 5).unwrap();
        assert_eq!(ctx.node.id(), "f-lib");
        assert!(!ctx.lineage.is_empty()); // should have parent dirs
        assert!(!ctx.children.is_empty()); // should have leaf children
    }

    #[test]
    fn selector_for_node_round_trips() {
        let graph = fixture_graph();
        let svc = GraphContextService::new(&graph);
        let sel_str = "file:src/lib.rs";
        let sel: Selector = sel_str.parse().unwrap();
        let node = svc.resolve_selector(&sel).unwrap();
        let generated = svc.selector_for_node(node);
        assert_eq!(generated, sel_str);
    }
}
