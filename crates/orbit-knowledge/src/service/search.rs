use crate::graph::navigator::GraphNodeRef;

use super::{GraphContextService, SearchResult};

struct SearchCriteria<'q> {
    query_lower: &'q str,
    browse: bool,
    node_types: Option<&'q [&'q str]>,
    location_prefix: Option<&'q str>,
    kind_filter: Option<&'q str>,
    limit: usize,
}

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
        let criteria = SearchCriteria {
            query_lower: &query_lower,
            browse: query_lower.is_empty(),
            node_types,
            location_prefix,
            kind_filter,
            limit,
        };
        let mut total = 0usize;
        let mut results = Vec::new();

        for dir in &self.graph.dirs {
            self.collect_search_match(
                GraphNodeRef::Dir(dir),
                "dir",
                &criteria,
                &mut total,
                &mut results,
            );
        }
        for file in &self.graph.files {
            self.collect_search_match(
                GraphNodeRef::File(file),
                "file",
                &criteria,
                &mut total,
                &mut results,
            );
        }
        for leaf in &self.graph.leaves {
            self.collect_search_match(
                GraphNodeRef::Leaf(leaf),
                "symbol",
                &criteria,
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
        criteria: &SearchCriteria<'_>,
        total: &mut usize,
        results: &mut Vec<GraphNodeRef<'a>>,
    ) {
        if !self.node_matches(node, node_type, criteria) {
            return;
        }

        *total += 1;
        if results.len() < criteria.limit {
            results.push(node);
        }
    }

    fn node_matches(
        &self,
        node: GraphNodeRef<'a>,
        node_type: &str,
        criteria: &SearchCriteria<'_>,
    ) -> bool {
        if let Some(types) = criteria.node_types
            && !types.contains(&node_type)
        {
            return false;
        }
        if let Some(prefix) = criteria.location_prefix
            && !node.location().starts_with(prefix)
        {
            return false;
        }
        if let Some(kind_filter) = criteria.kind_filter {
            match node {
                GraphNodeRef::Leaf(leaf) if leaf.kind.to_string() == kind_filter => {}
                GraphNodeRef::Leaf(_) => return false,
                _ => return false,
            }
        }

        criteria.browse
            || node
                .base()
                .name
                .to_lowercase()
                .contains(criteria.query_lower)
            || node
                .location()
                .to_lowercase()
                .contains(criteria.query_lower)
    }
}
