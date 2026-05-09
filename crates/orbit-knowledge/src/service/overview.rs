use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BinaryHeap, HashMap};

use crate::graph::navigator::GraphNodeRef;

use super::{
    FileOverview, GraphContextService, GraphOverview, GraphOverviewSummary, SymbolBrief,
    TopFileEntry,
};

impl<'a> GraphContextService<'a> {
    /// Build an aggregate overview of the graph, optionally scoped by location prefix.
    pub fn overview(&self, location_prefix: Option<&str>) -> GraphOverview {
        let in_scope = |location: &str| {
            location_prefix
                .map(|prefix| location.starts_with(prefix))
                .unwrap_or(true)
        };

        let mut total_dirs = 0usize;
        let mut total_files = 0usize;
        let mut total_symbols = 0usize;
        let mut languages: HashMap<String, usize> = HashMap::new();
        let mut symbol_kinds: HashMap<String, usize> = HashMap::new();
        let mut file_overviews = Vec::new();

        for dir in &self.graph.dirs {
            if in_scope(&dir.base.location) {
                total_dirs += 1;
            }
        }

        let mut file_leaves: HashMap<&str, Vec<SymbolBrief>> = HashMap::new();
        for leaf in &self.graph.leaves {
            if !in_scope(&leaf.base.location) {
                continue;
            }
            total_symbols += 1;
            let kind = leaf.kind.to_string();
            *symbol_kinds.entry(kind.clone()).or_default() += 1;
            let file_path = leaf
                .base
                .location
                .split_once('#')
                .map(|(path, _)| path)
                .unwrap_or(&leaf.base.location);
            file_leaves.entry(file_path).or_default().push(SymbolBrief {
                name: leaf.base.name.clone(),
                kind,
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
                path: file.base.location.clone(),
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
}

impl GraphOverview {
    /// Select the most symbol-dense files with deterministic tie-breaking.
    pub fn top_files(&self, limit: usize) -> Vec<TopFileEntry> {
        if limit == 0 {
            return Vec::new();
        }

        let heap_capacity = limit.min(self.files.len()).saturating_add(1);
        let mut heap: BinaryHeap<Reverse<FileOverviewRank<'_>>> =
            BinaryHeap::with_capacity(heap_capacity);
        for file in &self.files {
            heap.push(Reverse(FileOverviewRank(file)));
            if heap.len() > limit {
                heap.pop();
            }
        }

        let mut files: Vec<&FileOverview> = heap
            .into_iter()
            .map(|Reverse(FileOverviewRank(file))| file)
            .collect();
        files.sort_by(|left, right| compare_file_overview_rank(left, right));
        files
            .into_iter()
            .map(FileOverview::top_file_entry)
            .collect()
    }
}

#[derive(Clone, Copy)]
struct FileOverviewRank<'a>(&'a FileOverview);

impl PartialEq for FileOverviewRank<'_> {
    fn eq(&self, other: &Self) -> bool {
        compare_file_overview_rank(self.0, other.0) == Ordering::Equal
    }
}

impl Eq for FileOverviewRank<'_> {}

impl PartialOrd for FileOverviewRank<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FileOverviewRank<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_file_overview_rank(self.0, other.0).reverse()
    }
}

/// Build a compact overview summary suitable for broad repo orientation.
pub fn compact_from_overview(
    overview: &GraphOverview,
    location_prefix: Option<&str>,
    hint: &str,
) -> GraphOverviewSummary {
    let mut dir_file_counts = BTreeMap::new();
    for file in &overview.files {
        let key = top_level_dir_key(&file.path, location_prefix);
        *dir_file_counts.entry(key).or_insert(0) += 1;
    }

    GraphOverviewSummary {
        total_dirs: overview.total_dirs,
        total_files: overview.total_files,
        total_symbols: overview.total_symbols,
        languages: overview.languages.clone(),
        symbol_kinds: overview.symbol_kinds.clone(),
        dir_file_counts,
        top_files: overview.top_files(10),
        hint: hint.to_string(),
    }
}

impl FileOverview {
    fn top_file_entry(&self) -> TopFileEntry {
        TopFileEntry {
            selector: self.selector.clone(),
            name: self.name.clone(),
            symbol_count: self.symbol_count,
        }
    }
}

fn compare_file_overview_rank(left: &FileOverview, right: &FileOverview) -> Ordering {
    right
        .symbol_count
        .cmp(&left.symbol_count)
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| left.selector.cmp(&right.selector))
        .then_with(|| left.name.cmp(&right.name))
}

fn top_level_dir_key(path: &str, location_prefix: Option<&str>) -> String {
    let relative = location_prefix
        .and_then(|prefix| path.strip_prefix(prefix))
        .unwrap_or(path)
        .trim_start_matches('/');

    relative
        .split_once('/')
        .map(|(segment, _)| segment)
        .filter(|segment| !segment.is_empty())
        .unwrap_or(".")
        .to_string()
}

#[cfg(test)]
mod tests {
    use crate::graph::nodes::{
        BaseNodeFields, CodebaseGraphV1, DirNode, FileNode, LeafKind, LeafNode,
    };

    use super::*;

    #[test]
    fn top_files_matches_full_sort_for_large_fixture() {
        let graph = large_overview_graph();
        assert!(graph.files.len() >= 1000);

        let service = GraphContextService::new(&graph);
        let overview = service.overview(None);
        let expected = full_sort_top_files(&overview, 10);

        assert_eq!(overview.top_files(10), expected);
        assert_eq!(expected[0].selector, "file:src/top_tie_a.rs");
        assert_eq!(expected[1].selector, "file:src/top_tie_b.rs");
        assert_eq!(expected[0].symbol_count, expected[1].symbol_count);
    }

    fn full_sort_top_files(overview: &GraphOverview, limit: usize) -> Vec<TopFileEntry> {
        let mut files: Vec<&FileOverview> = overview.files.iter().collect();
        files.sort_by(|left, right| {
            right
                .symbol_count
                .cmp(&left.symbol_count)
                .then_with(|| left.path.cmp(&right.path))
                .then_with(|| left.selector.cmp(&right.selector))
                .then_with(|| left.name.cmp(&right.name))
        });
        files
            .into_iter()
            .take(limit)
            .map(FileOverview::top_file_entry)
            .collect()
    }

    fn large_overview_graph() -> CodebaseGraphV1 {
        let root_id = "dir:.".to_string();
        let mut fixture = vec![
            ("src/top_tie_a.rs".to_string(), 200),
            ("src/top_tie_b.rs".to_string(), 200),
            ("src/top_001.rs".to_string(), 199),
            ("src/top_002.rs".to_string(), 198),
            ("src/top_003.rs".to_string(), 197),
            ("src/top_004.rs".to_string(), 196),
            ("src/top_005.rs".to_string(), 195),
            ("src/top_006.rs".to_string(), 194),
            ("src/top_007.rs".to_string(), 193),
            ("src/top_008.rs".to_string(), 192),
        ];
        for index in 0..1000 {
            fixture.push((format!("src/bulk_{index:04}.rs"), index % 50));
        }

        let mut file_children = Vec::with_capacity(fixture.len());
        let mut files = Vec::with_capacity(fixture.len());
        let mut leaves = Vec::new();

        for (file_index, (path, symbol_count)) in fixture.into_iter().enumerate() {
            let file_id = format!("file:{path}");
            let file_name = path.rsplit('/').next().unwrap_or(path.as_str()).to_string();
            let mut leaf_children = Vec::with_capacity(symbol_count);

            for symbol_index in 0..symbol_count {
                let symbol_name = format!("symbol_{file_index}_{symbol_index}");
                let leaf_location = format!("{path}#{symbol_name}");
                let leaf_id = format!("symbol:{leaf_location}:function");
                leaf_children.push(leaf_id.clone());
                leaves.push(LeafNode {
                    base: base_node(
                        &leaf_id,
                        &symbol_name,
                        &leaf_location,
                        "rust",
                        Some(&file_id),
                    ),
                    kind: LeafKind::Function,
                    source: String::new(),
                    source_blob_hash: None,
                    source_hash: None,
                    file_hash_at_capture: None,
                    history: Vec::new(),
                    input_signature: Vec::new(),
                    output_signature: Vec::new(),
                    start_line: None,
                    end_line: None,
                    children: Vec::new(),
                });
            }

            file_children.push(file_id.clone());
            files.push(FileNode {
                base: base_node(&file_id, &file_name, &path, "rust", Some(&root_id)),
                extension: Some("rs".to_string()),
                source_blob_hash: None,
                source: String::new(),
                imports: Vec::new(),
                exports: Vec::new(),
                re_exports: Vec::new(),
                leaf_children,
            });
        }

        CodebaseGraphV1 {
            root_dir_id: root_id.clone(),
            dirs: vec![DirNode {
                base: base_node(&root_id, ".", ".", "", None),
                dir_children: Vec::new(),
                file_children,
            }],
            files,
            leaves,
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
