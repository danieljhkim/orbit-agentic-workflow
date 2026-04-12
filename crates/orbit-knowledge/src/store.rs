use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use orbit_types::OrbitError;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::error::KnowledgeError;
use crate::selector::{Selector, SelectorLookupKey};
use crate::working_graph::{WorkingGraph, WorkingLeaf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeEntryKind {
    Dir,
    File,
    Leaf,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgePackEntry {
    pub selector: String,
    pub resolved_id: Option<String>,
    pub kind: KnowledgeEntryKind,
    pub content: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgePack {
    pub knowledge_dir: String,
    pub manifest_generated_at: String,
    pub unresolved_selectors: Vec<String>,
    pub total_nodes: usize,
    pub entries: Vec<KnowledgePackEntry>,
}

const TASK_WORKING_GRAPH_DIR: &str = "knowledge/working";

/// Leaf data extracted from a graph object, used to initialize a WorkingGraph.
#[derive(Debug, Clone)]
pub struct LeafData {
    pub file_path: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub source: String,
    pub source_hash: String,
    pub parent_qualified_name: Option<String>,
    pub children_qualified_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeStore {
    knowledge_dir: PathBuf,
    manifest: ManifestFile,
    graph_index: GraphIndexFile,
    selector_index: HashMap<SelectorLookupKey, String>,
}

impl KnowledgeStore {
    pub fn open(knowledge_dir: &Path) -> Result<Self, KnowledgeError> {
        if !knowledge_dir.is_dir() {
            return Err(KnowledgeError::knowledge_unavailable(format!(
                "knowledge directory does not exist: {}",
                knowledge_dir.display()
            )));
        }

        let manifest_path = knowledge_dir.join("manifest.json");
        let manifest: ManifestFile = read_json_file(&manifest_path).map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!(
                "manifest.json is unavailable or invalid at {}: {error}",
                manifest_path.display()
            ))
        })?;

        let current_ref_path = knowledge_dir.join("graph/refs/current.json");
        let current_ref: CurrentRefFile = read_json_file(&current_ref_path).map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!(
                "graph reference is unavailable or invalid at {}: {error}",
                current_ref_path.display()
            ))
        })?;

        let graph_index_path = knowledge_dir.join(&current_ref.index);
        let graph_index: GraphIndexFile = read_json_file(&graph_index_path).map_err(|error| {
            KnowledgeError::knowledge_unavailable(format!(
                "graph index is unavailable or invalid at {}: {error}",
                graph_index_path.display()
            ))
        })?;

        let mut selector_index = HashMap::new();
        for (node_id, entry) in &graph_index.nodes {
            let key = match entry.node_type.as_str() {
                "dir" => SelectorLookupKey::Dir(entry.location.clone()),
                "file" => SelectorLookupKey::File(entry.location.clone()),
                "leaf" => SelectorLookupKey::Symbol(
                    entry.location.clone(),
                    entry.kind.clone().ok_or_else(|| {
                        KnowledgeError::invalid_data(format!(
                            "leaf index entry `{node_id}` is missing a `kind`"
                        ))
                    })?,
                ),
                other => {
                    return Err(KnowledgeError::invalid_data(format!(
                        "unsupported graph node type `{other}` for `{node_id}`"
                    )));
                }
            };
            selector_index.insert(key, node_id.clone());
        }

        Ok(Self {
            knowledge_dir: knowledge_dir.to_path_buf(),
            manifest,
            graph_index,
            selector_index,
        })
    }

    pub fn is_available(knowledge_dir: &Path) -> bool {
        Self::open(knowledge_dir).is_ok()
    }

    /// Iterate all leaf entries, loading graph objects to extract detailed data.
    ///
    /// Used by `WorkingGraph::from_store()` to populate the in-memory working graph.
    pub(crate) fn leaf_data(&self) -> Vec<(SelectorLookupKey, LeafData)> {
        let mut result = Vec::new();
        let mut object_cache = HashMap::<String, Value>::new();
        let mut blob_cache = HashMap::<String, String>::new();

        for entry in self.graph_index.nodes.values() {
            if entry.node_type != "leaf" {
                continue;
            }
            let kind = match &entry.kind {
                Some(k) => k.clone(),
                None => continue,
            };

            let object =
                match read_graph_object(&self.knowledge_dir, &entry.object_hash, &mut object_cache)
                {
                    Ok(v) => v,
                    Err(_) => continue,
                };

            let node = match object.get("node") {
                Some(n) => n,
                None => continue,
            };

            let start_line = node.get("start_line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let end_line = node.get("end_line").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let name = node
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let source_hash = node
                .get("source_hash")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source = extract_leaf_source(&self.knowledge_dir, &object, &mut blob_cache)
                .ok()
                .flatten()
                .unwrap_or_default();

            let (file_path, qualified_name) = if let Some((p, s)) = entry.location.split_once('#') {
                (p.to_string(), s.to_string())
            } else {
                continue;
            };

            let children: Vec<String> = node
                .get("children")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let key = SelectorLookupKey::Symbol(entry.location.clone(), kind.clone());
            result.push((
                key,
                LeafData {
                    file_path,
                    name,
                    qualified_name,
                    kind,
                    start_line,
                    end_line,
                    source,
                    source_hash,
                    parent_qualified_name: None,
                    children_qualified_names: children,
                },
            ));
        }

        result
    }

    pub fn pack(&self, selectors: &[Selector]) -> Result<KnowledgePack, KnowledgeError> {
        let mut object_cache = HashMap::<String, Value>::new();
        let mut blob_cache = HashMap::<String, String>::new();
        let mut entries = Vec::with_capacity(selectors.len());
        let mut unresolved_selectors = Vec::new();

        for selector in selectors {
            let selector_string = selector.to_string();
            let Some(node_id) = self.selector_index.get(&selector.lookup_key()).cloned() else {
                unresolved_selectors.push(selector_string.clone());
                entries.push(KnowledgePackEntry {
                    selector: selector_string,
                    resolved_id: None,
                    kind: KnowledgeEntryKind::Unresolved,
                    content: None,
                    source: None,
                });
                continue;
            };

            let index_entry = self.graph_index.nodes.get(&node_id).ok_or_else(|| {
                KnowledgeError::invalid_data(format!(
                    "graph index entry disappeared for `{node_id}`"
                ))
            })?;

            let content = read_graph_object(
                &self.knowledge_dir,
                &index_entry.object_hash,
                &mut object_cache,
            )?;
            let source = if index_entry.node_type == "leaf" {
                extract_leaf_source(&self.knowledge_dir, &content, &mut blob_cache)?
            } else {
                None
            };
            let kind = match index_entry.node_type.as_str() {
                "dir" => KnowledgeEntryKind::Dir,
                "file" => KnowledgeEntryKind::File,
                "leaf" => KnowledgeEntryKind::Leaf,
                other => {
                    return Err(KnowledgeError::invalid_data(format!(
                        "unsupported graph node type `{other}` for `{node_id}`"
                    )));
                }
            };

            entries.push(KnowledgePackEntry {
                selector: selector_string,
                resolved_id: Some(node_id),
                kind,
                content: Some(content),
                source,
            });
        }

        let total_nodes = entries
            .iter()
            .filter(|entry| entry.resolved_id.is_some())
            .count();
        Ok(KnowledgePack {
            knowledge_dir: self.knowledge_dir.display().to_string(),
            manifest_generated_at: self.manifest.generated_at.clone(),
            unresolved_selectors,
            total_nodes,
            entries,
        })
    }
}

pub fn task_working_graph_state_path(
    orbit_root: Option<&Path>,
    task_id: Option<&str>,
) -> Option<PathBuf> {
    let orbit_root = orbit_root?;
    let task_id = task_id?.trim();
    if task_id.is_empty() {
        return None;
    }
    Some(
        orbit_root
            .join(TASK_WORKING_GRAPH_DIR)
            .join(format!("{task_id}.json")),
    )
}

pub fn load_task_working_graph(
    orbit_root: Option<&Path>,
    task_id: Option<&str>,
) -> Result<Option<WorkingGraph>, OrbitError> {
    let Some(path) = task_working_graph_state_path(orbit_root, task_id) else {
        return Ok(None);
    };
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .map_err(|error| OrbitError::Execution(format!("read {}: {error}", path.display())))?;
    let graph = serde_json::from_str(&raw).map_err(|error| {
        OrbitError::Execution(format!(
            "parse task working graph state {}: {error}",
            path.display()
        ))
    })?;
    Ok(Some(graph))
}

pub fn save_task_working_graph(
    orbit_root: Option<&Path>,
    task_id: Option<&str>,
    graph: &WorkingGraph,
) -> Result<(), OrbitError> {
    let Some(path) = task_working_graph_state_path(orbit_root, task_id) else {
        return Ok(());
    };

    let parent = path
        .parent()
        .ok_or_else(|| OrbitError::Execution(format!("no parent dir for {}", path.display())))?;
    fs::create_dir_all(parent)
        .map_err(|error| OrbitError::Execution(format!("create {}: {error}", parent.display())))?;

    let tmp_path = parent.join(format!(
        ".{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("working-graph")
    ));
    let payload = serde_json::to_string_pretty(graph).map_err(|error| {
        OrbitError::Execution(format!(
            "serialize task working graph state {}: {error}",
            path.display()
        ))
    })?;
    fs::write(&tmp_path, format!("{payload}\n"))
        .map_err(|error| OrbitError::Execution(format!("write {}: {error}", tmp_path.display())))?;
    fs::rename(&tmp_path, &path).map_err(|error| {
        OrbitError::Execution(format!(
            "rename {} -> {}: {error}",
            tmp_path.display(),
            path.display()
        ))
    })?;
    Ok(())
}

pub fn overlay_pack_with_working_graph(
    mut pack: KnowledgePack,
    selectors: &[Selector],
    graph: &WorkingGraph,
) -> KnowledgePack {
    for (entry, selector) in pack.entries.iter_mut().zip(selectors.iter()) {
        let Some(working_leaf) = graph.resolve_leaf(selector) else {
            continue;
        };

        entry.kind = KnowledgeEntryKind::Leaf;
        entry.source = Some(working_leaf.source.clone());
        entry.resolved_id = entry
            .resolved_id
            .clone()
            .or_else(|| Some(selector.to_string()));
        entry.content = Some(match entry.content.take() {
            Some(content) => overlay_leaf_content(content, working_leaf),
            None => build_working_leaf_content(working_leaf),
        });
    }

    pack.unresolved_selectors = pack
        .entries
        .iter()
        .filter(|entry| entry.resolved_id.is_none())
        .map(|entry| entry.selector.clone())
        .collect();
    pack.total_nodes = pack
        .entries
        .iter()
        .filter(|entry| entry.resolved_id.is_some())
        .count();
    pack
}

pub fn pack_from_working_graph(
    knowledge_dir: &Path,
    selectors: &[Selector],
    graph: &WorkingGraph,
) -> KnowledgePack {
    let mut entries = Vec::with_capacity(selectors.len());

    for selector in selectors {
        let selector_string = selector.to_string();
        if let Some(working_leaf) = graph.resolve_leaf(selector) {
            entries.push(KnowledgePackEntry {
                selector: selector_string.clone(),
                resolved_id: Some(selector_string),
                kind: KnowledgeEntryKind::Leaf,
                content: Some(build_working_leaf_content(working_leaf)),
                source: Some(working_leaf.source.clone()),
            });
        } else {
            entries.push(KnowledgePackEntry {
                selector: selector_string,
                resolved_id: None,
                kind: KnowledgeEntryKind::Unresolved,
                content: None,
                source: None,
            });
        }
    }

    let unresolved_selectors = entries
        .iter()
        .filter(|entry| entry.resolved_id.is_none())
        .map(|entry| entry.selector.clone())
        .collect();
    let total_nodes = entries
        .iter()
        .filter(|entry| entry.resolved_id.is_some())
        .count();

    KnowledgePack {
        knowledge_dir: knowledge_dir.display().to_string(),
        manifest_generated_at: String::new(),
        unresolved_selectors,
        total_nodes,
        entries,
    }
}

fn overlay_leaf_content(mut content: Value, leaf: &WorkingLeaf) -> Value {
    let Some(node) = content.get_mut("node").and_then(Value::as_object_mut) else {
        return build_working_leaf_content(leaf);
    };

    node.insert("name".to_string(), Value::String(leaf.name.clone()));
    node.insert("kind".to_string(), Value::String(leaf.kind.clone()));
    node.insert(
        "location".to_string(),
        Value::String(format!("{}#{}", leaf.file_path, leaf.qualified_name)),
    );
    node.insert("start_line".to_string(), json!(leaf.start_line));
    node.insert("end_line".to_string(), json!(leaf.end_line));
    node.insert("source".to_string(), Value::String(leaf.source.clone()));
    node.insert(
        "source_hash".to_string(),
        Value::String(leaf.source_hash.clone()),
    );
    node.insert("children".to_string(), json!(leaf.children_qualified_names));
    if let Some(parent) = &leaf.parent_qualified_name {
        node.insert("parent".to_string(), Value::String(parent.clone()));
    } else {
        node.remove("parent");
    }

    content
}

fn build_working_leaf_content(leaf: &WorkingLeaf) -> Value {
    json!({
        "node": {
            "name": leaf.name.clone(),
            "qualified_name": leaf.qualified_name.clone(),
            "kind": leaf.kind.clone(),
            "location": format!("{}#{}", leaf.file_path, leaf.qualified_name),
            "language": Path::new(&leaf.file_path)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default(),
            "start_line": leaf.start_line,
            "end_line": leaf.end_line,
            "source": leaf.source.clone(),
            "source_hash": leaf.source_hash.clone(),
            "parent": leaf.parent_qualified_name.clone(),
            "children": leaf.children_qualified_names.clone(),
        }
    })
}

fn read_graph_object(
    knowledge_dir: &Path,
    object_hash: &str,
    cache: &mut HashMap<String, Value>,
) -> Result<Value, KnowledgeError> {
    if let Some(value) = cache.get(object_hash) {
        return Ok(value.clone());
    }

    let path = knowledge_dir
        .join("graph/objects")
        .join(&object_hash[..2])
        .join(format!("{object_hash}.json"));
    let value: Value = read_json_file(&path).map_err(|error| {
        KnowledgeError::knowledge_unavailable(format!(
            "graph object `{object_hash}` is unavailable at {}: {error}",
            path.display()
        ))
    })?;
    cache.insert(object_hash.to_string(), value.clone());
    Ok(value)
}

fn extract_leaf_source(
    knowledge_dir: &Path,
    object: &Value,
    blob_cache: &mut HashMap<String, String>,
) -> Result<Option<String>, KnowledgeError> {
    if let Some(source) = object
        .get("node")
        .and_then(|node| node.get("source"))
        .and_then(Value::as_str)
        .filter(|source| !source.is_empty())
    {
        return Ok(Some(source.to_string()));
    }

    let Some(blob_hash) = object
        .get("node")
        .and_then(|node| node.get("source_blob_hash"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };

    if let Some(source) = blob_cache.get(blob_hash) {
        return Ok(Some(source.clone()));
    }

    let path = knowledge_dir
        .join("graph/blobs")
        .join(&blob_hash[..2])
        .join(format!("{blob_hash}.txt"));
    let source = fs::read_to_string(&path).map_err(|error| {
        KnowledgeError::knowledge_unavailable(format!(
            "graph blob `{blob_hash}` is unavailable at {}: {error}",
            path.display()
        ))
    })?;
    blob_cache.insert(blob_hash.to_string(), source.clone());
    Ok(Some(source))
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&raw).map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    generated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentRefFile {
    index: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GraphIndexFile {
    nodes: HashMap<String, GraphIndexEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct GraphIndexEntry {
    object_hash: String,
    node_type: String,
    location: String,
    kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde_json::json;

    use super::{
        KnowledgeEntryKind, KnowledgePack, KnowledgePackEntry, KnowledgeStore,
        task_working_graph_state_path,
    };
    use crate::error::KnowledgeError;
    use crate::selector::{Selector, SelectorParseError};

    fn fixture_knowledge_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../orbit-tools/tests/fixtures/knowledge")
            .join(".orbit/knowledge")
    }

    #[test]
    fn parses_supported_selector_forms() {
        assert_eq!(
            "dir:crates/orbit-tools/src".parse::<Selector>().unwrap(),
            Selector::Dir {
                path: "crates/orbit-tools/src".to_string()
            }
        );
        assert_eq!(
            "file:crates/orbit-tools/src/lib.rs"
                .parse::<Selector>()
                .unwrap(),
            Selector::File {
                path: "crates/orbit-tools/src/lib.rs".to_string()
            }
        );
        assert_eq!(
            "symbol:crates/orbit-tools/src/lib.rs#register_builtins:function"
                .parse::<Selector>()
                .unwrap(),
            Selector::Symbol {
                path: "crates/orbit-tools/src/lib.rs".to_string(),
                symbol: "register_builtins".to_string(),
                kind: "function".to_string(),
            }
        );
    }

    #[test]
    fn rejects_malformed_selectors() {
        assert_eq!(
            "".parse::<Selector>().unwrap_err(),
            SelectorParseError {
                selector: "".to_string(),
                reason: "selectors must start with `dir:`, `file:`, or `symbol:`".to_string(),
            }
        );
        assert_eq!(
            "file:".parse::<Selector>().unwrap_err(),
            SelectorParseError {
                selector: "file:".to_string(),
                reason: "file selector path must not be empty".to_string(),
            }
        );
        assert_eq!(
            "symbol:crates/orbit-tools/src/lib.rs#register_builtins"
                .parse::<Selector>()
                .unwrap_err(),
            SelectorParseError {
                selector: "symbol:crates/orbit-tools/src/lib.rs#register_builtins".to_string(),
                reason: "symbol selectors must use `symbol:<path>#<symbol>:<kind>`".to_string(),
            }
        );
    }

    #[test]
    fn open_reports_missing_knowledge_directory() {
        let missing = fixture_knowledge_dir().join("missing");

        let error = KnowledgeStore::open(&missing).unwrap_err();

        assert_eq!(
            error,
            KnowledgeError {
                kind: "knowledge_unavailable".to_string(),
                reason: format!("knowledge directory does not exist: {}", missing.display()),
            }
        );
    }

    #[test]
    fn open_reports_corrupt_manifest() {
        let corrupt_dir = tempfile::tempdir().expect("tempdir");
        let knowledge_dir = corrupt_dir.path().join(".orbit/knowledge");
        std::fs::create_dir_all(&knowledge_dir).expect("knowledge dir");
        std::fs::write(knowledge_dir.join("manifest.json"), "{not-json").expect("manifest");

        let error = KnowledgeStore::open(&knowledge_dir).unwrap_err();

        assert_eq!(error.kind, "knowledge_unavailable");
        assert!(!error.reason.is_empty());
        assert!(error.reason.contains("manifest.json"));
    }

    #[test]
    fn availability_checks_manifest_and_index() {
        assert!(KnowledgeStore::is_available(&fixture_knowledge_dir()));
        assert!(!KnowledgeStore::is_available(Path::new(
            "/tmp/definitely-missing-orbit"
        )));
    }

    #[test]
    fn task_working_graph_state_path_is_under_knowledge() {
        let root = Path::new("/tmp/test-orbit");
        let path = task_working_graph_state_path(Some(root), Some("T-001")).expect("path");

        assert!(path.ends_with("knowledge/working/T-001.json"));
        assert!(!path.to_string_lossy().contains("runtime"));
    }

    #[test]
    fn pack_returns_exact_fixture_output() {
        let store = KnowledgeStore::open(&fixture_knowledge_dir()).expect("store");
        let selectors = Selector::parse_many(&[
            "file:crates/orbit-tools/src/lib.rs".to_string(),
            "symbol:crates/orbit-tools/src/lib.rs#register_builtins:function".to_string(),
        ])
        .expect("selectors");

        let pack = store.pack(&selectors).expect("pack");

        assert_eq!(
            pack,
            KnowledgePack {
                knowledge_dir: fixture_knowledge_dir().display().to_string(),
                manifest_generated_at: "2026-04-09T06:06:39Z".to_string(),
                unresolved_selectors: vec![],
                total_nodes: 2,
                entries: vec![
                    KnowledgePackEntry {
                        selector: "file:crates/orbit-tools/src/lib.rs".to_string(),
                        resolved_id: Some("node-file-lib".to_string()),
                        kind: KnowledgeEntryKind::File,
                        content: Some(json!({
                            "schema_version": 1,
                            "object_type": "graph_node",
                            "node_type": "file",
                            "node": {
                                "description": "Orbit tool registry facade.",
                                "exports": ["ToolRegistry"],
                                "extension": "rs",
                                "id": "node-file-lib",
                                "identity_key": "file:crates/orbit-tools/src/lib.rs",
                                "imports": ["serde_json::Value"],
                                "is_locked": false,
                                "language": "rust",
                                "leaf_children": ["node-leaf-register-builtins"],
                                "lineage_locked": false,
                                "location": "crates/orbit-tools/src/lib.rs",
                                "lock_owner": null,
                                "lock_reason": "",
                                "name": "lib.rs",
                                "parent_id": "node-dir-tools-src",
                                "source_blob_hash": "6666666666666666666666666666666666666666666666666666666666666666"
                            },
                            "child_object_hashes": {
                                "node-leaf-register-builtins": "3333333333333333333333333333333333333333333333333333333333333333"
                            }
                        })),
                        source: None,
                    },
                    KnowledgePackEntry {
                        selector: "symbol:crates/orbit-tools/src/lib.rs#register_builtins:function"
                            .to_string(),
                        resolved_id: Some("node-leaf-register-builtins".to_string()),
                        kind: KnowledgeEntryKind::Leaf,
                        content: Some(json!({
                            "schema_version": 1,
                            "object_type": "graph_node",
                            "node_type": "leaf",
                            "node": {
                                "children": [],
                                "description": "Registers the built-in orbit tools.",
                                "end_line": 22,
                                "file_hash_at_capture": "7777777777777777777777777777777777777777777777777777777777777777",
                                "history": [],
                                "id": "node-leaf-register-builtins",
                                "identity_key": "symbol:crates/orbit-tools/src/lib.rs#register_builtins:function",
                                "input_signature": [],
                                "is_locked": false,
                                "kind": "function",
                                "language": "rust",
                                "lineage_locked": false,
                                "location": "crates/orbit-tools/src/lib.rs#register_builtins",
                                "lock_owner": null,
                                "lock_reason": "",
                                "name": "register_builtins",
                                "output_signature": [],
                                "parent_id": "node-file-lib",
                                "source": "",
                                "source_blob_hash": "5555555555555555555555555555555555555555555555555555555555555555",
                                "source_hash": "8888888888888888888888888888888888888888888888888888888888888888",
                                "start_line": 11
                            },
                            "child_object_hashes": {}
                        })),
                        source: Some(
                            "pub fn register_builtins(registry: &mut ToolRegistry) {\n    fs::register(registry);\n    git::register(registry);\n}\n"
                                .to_string()
                        ),
                    },
                ],
            }
        );
    }

    #[test]
    fn pack_keeps_unresolved_selectors_out_of_error_path() {
        let store = KnowledgeStore::open(&fixture_knowledge_dir()).expect("store");
        let selectors = Selector::parse_many(&[
            "file:crates/orbit-tools/src/lib.rs".to_string(),
            "dir:crates/orbit-tools/src/missing".to_string(),
        ])
        .expect("selectors");

        let pack = store.pack(&selectors).expect("pack");

        assert_eq!(
            pack.unresolved_selectors,
            vec!["dir:crates/orbit-tools/src/missing".to_string()]
        );
        assert_eq!(pack.total_nodes, 1);
        assert_eq!(pack.entries[1].kind, KnowledgeEntryKind::Unresolved);
        assert_eq!(pack.entries[1].resolved_id, None);
        assert_eq!(pack.entries[1].content, None);
    }
}
