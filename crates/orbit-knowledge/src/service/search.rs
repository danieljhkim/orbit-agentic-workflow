use crate::graph::navigator::GraphNodeRef;

use super::{GraphContextService, SearchResult};

impl<'a> GraphContextService<'a> {
    pub fn search_with_total(
        &self,
        query: &str,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
        limit: usize,
    ) -> (usize, Vec<GraphNodeRef<'a>>) {
        let query_lower = query.to_lowercase();
        let browse = query_lower.is_empty();
        let mut total = 0usize;
        let mut results = Vec::new();

        for dir in &self.graph.dirs {
            self.collect_search_match(
                GraphNodeRef::Dir(dir),
                "dir",
                &query_lower,
                browse,
                node_types,
                location_prefix,
                kind_filter,
                limit,
                &mut total,
                &mut results,
            );
        }
        for file in &self.graph.files {
            self.collect_search_match(
                GraphNodeRef::File(file),
                "file",
                &query_lower,
                browse,
                node_types,
                location_prefix,
                kind_filter,
                limit,
                &mut total,
                &mut results,
            );
        }
        for leaf in &self.graph.leaves {
            self.collect_search_match(
                GraphNodeRef::Leaf(leaf),
                "symbol",
                &query_lower,
                browse,
                node_types,
                location_prefix,
                kind_filter,
                limit,
                &mut total,
                &mut results,
            );
        }

        (total, results)
    }

    pub fn search(
        &self,
        query: &str,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
        limit: usize,
    ) -> Vec<GraphNodeRef<'a>> {
        self.search_with_total(query, node_types, location_prefix, kind_filter, limit)
            .1
    }

    pub fn search_total(
        &self,
        query: &str,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
    ) -> usize {
        self.search_with_total(query, node_types, location_prefix, kind_filter, 0)
            .0
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
        let nodes = self
            .search_with_total(query, node_types, location_prefix, kind_filter, limit)
            .1;
        nodes
            .into_iter()
            .map(|node| {
                let selector = self.selector_for_node(node);
                let name = node.base().name.to_string();
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
                SearchResult {
                    selector,
                    name,
                    kind,
                    file,
                }
            })
            .collect()
    }

    fn collect_search_match(
        &self,
        node: GraphNodeRef<'a>,
        node_type: &str,
        query_lower: &str,
        browse: bool,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
        limit: usize,
        total: &mut usize,
        results: &mut Vec<GraphNodeRef<'a>>,
    ) {
        if !self.node_matches(
            node,
            node_type,
            query_lower,
            browse,
            node_types,
            location_prefix,
            kind_filter,
        ) {
            return;
        }

        *total += 1;
        if results.len() < limit {
            results.push(node);
        }
    }

    fn node_matches(
        &self,
        node: GraphNodeRef<'a>,
        node_type: &str,
        query_lower: &str,
        browse: bool,
        node_types: Option<&[&str]>,
        location_prefix: Option<&str>,
        kind_filter: Option<&str>,
    ) -> bool {
        if let Some(types) = node_types
            && !types.contains(&node_type)
        {
            return false;
        }
        if let Some(prefix) = location_prefix
            && !node.location().starts_with(prefix)
        {
            return false;
        }
        if let Some(kind_filter) = kind_filter {
            match node {
                GraphNodeRef::Leaf(leaf) if leaf.kind.to_string() == kind_filter => {}
                GraphNodeRef::Leaf(_) => return false,
                _ => return false,
            }
        }

        browse
            || node.base().name.to_lowercase().contains(query_lower)
            || node.location().to_lowercase().contains(query_lower)
    }
}
