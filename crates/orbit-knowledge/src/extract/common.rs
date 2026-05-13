// ORB-00013: Existing expect calls in this module document local invariants; keep the allow scoped while the workspace lint is ratcheted.
#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, HashMap, HashSet};

use sha2::{Digest, Sha256};

/// A single extracted leaf from a source file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedLeaf {
    pub qualified_name: String,
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub source: String,
    pub source_hash: String,
    /// For methods: the qualified_name of the parent impl/class block.
    pub parent_qualified_name: Option<String>,
    /// For impl/class blocks: qualified_names of child methods.
    pub children_qualified_names: Vec<String>,
    /// ATX heading depth (1–6) for `LeafKind::Section`. `None` for every other kind.
    /// Added T20260422-1540 alongside markdown/config/tabular extraction.
    pub depth: Option<u8>,
}

/// A file-level export discovered while extracting source.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractedExport {
    pub name: String,
    /// Source path for `pub use` re-exports. `None` means the symbol is defined
    /// in the file itself.
    pub source_path: Option<String>,
}

/// Result of extracting a single file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractionResult {
    pub leaves: Vec<ExtractedLeaf>,
    pub exports: Vec<ExtractedExport>,
}

pub fn compute_source_hash(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Build the identity key for a graph node: `"{node_type}:{location}:{kind}"`.
pub fn identity_key(node_type: &str, location: &str, kind: &str) -> String {
    format!("{node_type}:{location}:{kind}")
}

/// Deterministic node ID: `"{node_type}:{sha256_hex(identity_key)}"`.
pub fn node_id(node_type: &str, location: &str, kind: &str) -> String {
    let key = identity_key(node_type, location, kind);
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{node_type}:{digest}")
}

/// Build a leaf location string: `"{path}#{qualified_name}"`.
pub fn leaf_location(path: &str, qualified_name: &str) -> String {
    format!("{path}#{qualified_name}")
}

/// Ensure every `(qualified_name, kind)` pair is unique within one extractor result.
///
/// Language extractors should use semantic qualifiers first; this pass is the
/// deterministic safety net for overloads or future parser shapes that still
/// collide after extraction.
pub fn finalize_unique_qualified_names(leaves: &mut [ExtractedLeaf]) {
    let mut groups: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (index, leaf) in leaves.iter().enumerate() {
        groups
            .entry((leaf.qualified_name.clone(), leaf.kind.clone()))
            .or_default()
            .push(index);
    }

    let mut occupied: HashSet<(String, String)> = leaves
        .iter()
        .map(|leaf| (leaf.qualified_name.clone(), leaf.kind.clone()))
        .collect();
    let mut duplicate_originals: HashMap<String, Vec<usize>> = HashMap::new();

    for ((original_name, kind), indexes) in groups.iter_mut() {
        if indexes.len() < 2 {
            continue;
        }

        indexes.sort_by(|left, right| {
            let left_leaf = &leaves[*left];
            let right_leaf = &leaves[*right];
            (
                left_leaf.start_line,
                left_leaf.end_line,
                left_leaf.source_hash.as_str(),
                *left,
            )
                .cmp(&(
                    right_leaf.start_line,
                    right_leaf.end_line,
                    right_leaf.source_hash.as_str(),
                    *right,
                ))
        });

        duplicate_originals.insert(original_name.clone(), indexes.clone());
        let mut ordinal = 2usize;
        for index in indexes.iter().copied().skip(1) {
            let new_name = loop {
                let candidate = format!("{original_name}#{ordinal}");
                ordinal += 1;
                if !occupied.contains(&(candidate.clone(), kind.clone())) {
                    break candidate;
                }
            };
            occupied.insert((new_name.clone(), kind.clone()));
            leaves[index].qualified_name = new_name;
        }
    }

    if duplicate_originals.is_empty() {
        return;
    }

    let final_names: Vec<String> = leaves
        .iter()
        .map(|leaf| leaf.qualified_name.clone())
        .collect();

    for child_index in 0..leaves.len() {
        let Some(parent_name) = leaves[child_index].parent_qualified_name.clone() else {
            continue;
        };
        let Some(parent_indexes) = duplicate_originals.get(&parent_name) else {
            continue;
        };
        if let Some(parent_index) = containing_leaf(leaves, parent_indexes, child_index) {
            leaves[child_index].parent_qualified_name = Some(final_names[parent_index].clone());
        }
    }

    for parent_index in 0..leaves.len() {
        if leaves[parent_index].children_qualified_names.is_empty() {
            continue;
        }

        let mut seen_children_by_name: HashMap<String, usize> = HashMap::new();
        let children = leaves[parent_index].children_qualified_names.clone();
        let rewritten_children = children
            .into_iter()
            .map(|child_name| {
                let Some(child_indexes) = duplicate_originals.get(&child_name) else {
                    return child_name;
                };
                let scoped_children = contained_leaves(leaves, child_indexes, parent_index);
                if scoped_children.is_empty() {
                    return child_name;
                }
                let occurrence = seen_children_by_name.entry(child_name.clone()).or_default();
                let replacement_index = scoped_children
                    .get(*occurrence)
                    .copied()
                    .unwrap_or_else(|| *scoped_children.last().expect("non-empty"));
                *occurrence += 1;
                final_names[replacement_index].clone()
            })
            .collect();
        leaves[parent_index].children_qualified_names = rewritten_children;
    }
}

fn containing_leaf(
    leaves: &[ExtractedLeaf],
    candidates: &[usize],
    child_index: usize,
) -> Option<usize> {
    let child = &leaves[child_index];
    candidates
        .iter()
        .copied()
        .filter(|candidate_index| *candidate_index != child_index)
        .filter(|candidate_index| {
            let candidate = &leaves[*candidate_index];
            candidate.start_line <= child.start_line && child.end_line <= candidate.end_line
        })
        .min_by_key(|candidate_index| {
            let candidate = &leaves[*candidate_index];
            (
                candidate.end_line.saturating_sub(candidate.start_line),
                candidate.start_line,
                candidate.end_line,
                *candidate_index,
            )
        })
}

fn contained_leaves(
    leaves: &[ExtractedLeaf],
    candidates: &[usize],
    parent_index: usize,
) -> Vec<usize> {
    let parent = &leaves[parent_index];
    let mut contained: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|candidate_index| *candidate_index != parent_index)
        .filter(|candidate_index| {
            let candidate = &leaves[*candidate_index];
            parent.start_line <= candidate.start_line && candidate.end_line <= parent.end_line
        })
        .collect();
    contained.sort_by_key(|candidate_index| {
        let candidate = &leaves[*candidate_index];
        (
            candidate.start_line,
            candidate.end_line,
            candidate.source_hash.as_str(),
            *candidate_index,
        )
    });
    contained
}
