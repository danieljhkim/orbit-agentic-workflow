//! Content-addressed graph persistence for `.orbit/knowledge/graph/`.
//!
//! Reads and writes the same on-disk format as Python's `orbit_map/graph/store.py`.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::nodes::{CodebaseGraphV1, DirNode, FileNode, LeafNode};
use crate::error::KnowledgeError;

const GRAPH_STORE_SCHEMA_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Internal deserialization types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CurrentRef {
    index: String,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ByIdIndex {
    #[serde(default)]
    schema_version: u32,
    root_dir_id: String,
    #[serde(default)]
    root_graph_hash: String,
    #[serde(default)]
    root_object_hash: String,
    #[serde(default)]
    dirs: Vec<String>,
    #[serde(default)]
    files: Vec<String>,
    #[serde(default)]
    leaves: Vec<String>,
    nodes: HashMap<String, IndexEntry>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct IndexEntry {
    object_hash: String,
    node_type: String,
    location: String,
    #[serde(default)]
    identity_key: String,
    kind: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ObjectEnvelope {
    #[serde(default)]
    schema_version: u32,
    object_type: String,
    node_type: String,
    node: Value,
    #[serde(default)]
    child_object_hashes: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Public store
// ---------------------------------------------------------------------------

pub struct GraphObjectStore {
    graph_dir: PathBuf,
}

impl GraphObjectStore {
    pub fn new(graph_dir: impl Into<PathBuf>) -> Self {
        Self {
            graph_dir: graph_dir.into(),
        }
    }

    // -- directory helpers --------------------------------------------------

    fn refs_dir(&self) -> PathBuf {
        self.graph_dir.join("refs")
    }
    fn index_dir(&self) -> PathBuf {
        self.graph_dir.join("index")
    }
    fn objects_dir(&self) -> PathBuf {
        self.graph_dir.join("objects")
    }
    fn blobs_dir(&self) -> PathBuf {
        self.graph_dir.join("blobs")
    }
    fn object_path(&self, hash: &str) -> PathBuf {
        self.objects_dir()
            .join(&hash[..2])
            .join(format!("{hash}.json"))
    }
    fn blob_path(&self, hash: &str) -> PathBuf {
        self.blobs_dir()
            .join(&hash[..2])
            .join(format!("{hash}.txt"))
    }

    // -----------------------------------------------------------------------
    // Read path
    // -----------------------------------------------------------------------

    pub fn read_graph(&self) -> Result<CodebaseGraphV1, KnowledgeError> {
        // 1. Read refs/current.json
        let current_ref_path = self.refs_dir().join("current.json");
        let current_ref: CurrentRef = read_json_file(&current_ref_path)?;

        // 2. Resolve index path (strip leading "graph/" prefix if present)
        let index_rel = current_ref
            .index
            .strip_prefix("graph/")
            .unwrap_or(&current_ref.index);
        let index_path = self.graph_dir.join(index_rel);
        let index: ByIdIndex = read_json_file(&index_path)?;

        // 3. Read dir nodes
        let mut dirs = Vec::with_capacity(index.dirs.len());
        for node_id in &index.dirs {
            let entry = index.nodes.get(node_id).ok_or_else(|| {
                KnowledgeError::invalid_data(format!("dir node {node_id} missing from index"))
            })?;
            let envelope: ObjectEnvelope = read_json_file(&self.object_path(&entry.object_hash))?;
            let mut dir: DirNode = serde_json::from_value(envelope.node)
                .map_err(|e| KnowledgeError::invalid_data(format!("dir node parse: {e}")))?;
            dir.base.object_hash = Some(entry.object_hash.clone());
            dirs.push(dir);
        }

        // 4. Read file nodes
        let mut files = Vec::with_capacity(index.files.len());
        for node_id in &index.files {
            let entry = index.nodes.get(node_id).ok_or_else(|| {
                KnowledgeError::invalid_data(format!("file node {node_id} missing from index"))
            })?;
            let envelope: ObjectEnvelope = read_json_file(&self.object_path(&entry.object_hash))?;
            let mut file: FileNode = serde_json::from_value(envelope.node)
                .map_err(|e| KnowledgeError::invalid_data(format!("file node parse: {e}")))?;
            file.base.object_hash = Some(entry.object_hash.clone());
            files.push(file);
        }

        // 5. Read leaf nodes (hydrate source from blobs)
        let mut leaves = Vec::with_capacity(index.leaves.len());
        for node_id in &index.leaves {
            let entry = index.nodes.get(node_id).ok_or_else(|| {
                KnowledgeError::invalid_data(format!("leaf node {node_id} missing from index"))
            })?;
            let envelope: ObjectEnvelope = read_json_file(&self.object_path(&entry.object_hash))?;
            let mut leaf: LeafNode = serde_json::from_value(envelope.node)
                .map_err(|e| KnowledgeError::invalid_data(format!("leaf node parse: {e}")))?;
            leaf.base.object_hash = Some(entry.object_hash.clone());

            // Hydrate source from blob if present and source is empty
            if let Some(ref blob_hash) = leaf.source_blob_hash
                && leaf.source.is_empty()
            {
                leaf.source = self.read_blob(blob_hash)?;
            }

            leaves.push(leaf);
        }

        Ok(CodebaseGraphV1 {
            root_dir_id: index.root_dir_id,
            dirs,
            files,
            leaves,
        })
    }

    // -----------------------------------------------------------------------
    // Write path
    // -----------------------------------------------------------------------

    pub fn write_graph(&self, graph: &CodebaseGraphV1) -> Result<String, KnowledgeError> {
        self.ensure_dirs()?;

        let mut object_hashes: HashMap<String, String> = HashMap::new();
        let mut index_nodes: HashMap<String, Value> = HashMap::new();

        // 1. Write leaves (children first)
        for leaf in &graph.leaves {
            let mut node_json = serde_json::to_value(leaf)
                .map_err(|e| KnowledgeError::invalid_data(format!("leaf serialize: {e}")))?;

            // Strip object_hash from node data before hashing
            if let Value::Object(ref mut map) = node_json {
                map.remove("object_hash");
            }

            // Write source to blob, update blob hash, clear source in node JSON
            if !leaf.source.is_empty() {
                let bh = self.write_blob(&leaf.source)?;
                if let Value::Object(ref mut map) = node_json {
                    map.insert("source_blob_hash".to_string(), Value::String(bh));
                    map.insert("source".to_string(), Value::String(String::new()));
                }
            }

            let envelope = serde_json::json!({
                "schema_version": GRAPH_STORE_SCHEMA_VERSION,
                "object_type": "graph_node",
                "node_type": "leaf",
                "node": node_json,
                "child_object_hashes": {}
            });

            let object_hash = self.write_json_object(&envelope)?;
            object_hashes.insert(leaf.base.id.clone(), object_hash.clone());
            index_nodes.insert(
                leaf.base.id.clone(),
                serde_json::json!({
                    "object_hash": object_hash,
                    "node_type": "leaf",
                    "location": leaf.base.location,
                    "identity_key": leaf.base.identity_key,
                    "kind": leaf.kind,
                }),
            );
        }

        // 2. Write files
        for file in &graph.files {
            let mut node_json = serde_json::to_value(file)
                .map_err(|e| KnowledgeError::invalid_data(format!("file serialize: {e}")))?;
            if let Value::Object(ref mut map) = node_json {
                map.remove("object_hash");
            }

            let child_hashes: HashMap<String, String> = file
                .leaf_children
                .iter()
                .filter_map(|cid| object_hashes.get(cid).map(|h| (cid.clone(), h.clone())))
                .collect();

            let envelope = serde_json::json!({
                "schema_version": GRAPH_STORE_SCHEMA_VERSION,
                "object_type": "graph_node",
                "node_type": "file",
                "node": node_json,
                "child_object_hashes": child_hashes,
            });

            let object_hash = self.write_json_object(&envelope)?;
            object_hashes.insert(file.base.id.clone(), object_hash.clone());
            index_nodes.insert(
                file.base.id.clone(),
                serde_json::json!({
                    "object_hash": object_hash,
                    "node_type": "file",
                    "location": file.base.location,
                    "identity_key": file.base.identity_key,
                    "kind": null,
                }),
            );
        }

        // 3. Write dirs (children before parents -- simple single-pass)
        for dir in &graph.dirs {
            let mut node_json = serde_json::to_value(dir)
                .map_err(|e| KnowledgeError::invalid_data(format!("dir serialize: {e}")))?;
            if let Value::Object(ref mut map) = node_json {
                map.remove("object_hash");
            }

            let mut child_hashes: HashMap<String, String> = HashMap::new();
            for cid in &dir.dir_children {
                if let Some(h) = object_hashes.get(cid) {
                    child_hashes.insert(cid.clone(), h.clone());
                }
            }
            for cid in &dir.file_children {
                if let Some(h) = object_hashes.get(cid) {
                    child_hashes.insert(cid.clone(), h.clone());
                }
            }

            let envelope = serde_json::json!({
                "schema_version": GRAPH_STORE_SCHEMA_VERSION,
                "object_type": "graph_node",
                "node_type": "dir",
                "node": node_json,
                "child_object_hashes": child_hashes,
            });

            let object_hash = self.write_json_object(&envelope)?;
            object_hashes.insert(dir.base.id.clone(), object_hash.clone());
            index_nodes.insert(
                dir.base.id.clone(),
                serde_json::json!({
                    "object_hash": object_hash,
                    "node_type": "dir",
                    "location": dir.base.location,
                    "identity_key": dir.base.identity_key,
                    "kind": null,
                }),
            );
        }

        // 4. Root object hash
        let root_object_hash = object_hashes
            .get(&graph.root_dir_id)
            .cloned()
            .unwrap_or_default();

        // 5. Write root graph object
        let root_payload = serde_json::json!({
            "schema_version": GRAPH_STORE_SCHEMA_VERSION,
            "object_type": "codebase_graph",
            "root_dir_id": graph.root_dir_id,
            "root_object_hash": root_object_hash,
            "dirs": graph.dirs.iter().map(|d| &d.base.id).collect::<Vec<_>>(),
            "files": graph.files.iter().map(|f| &f.base.id).collect::<Vec<_>>(),
            "leaves": graph.leaves.iter().map(|l| &l.base.id).collect::<Vec<_>>(),
            "node_count": graph.dirs.len() + graph.files.len() + graph.leaves.len(),
        });
        let root_graph_hash = self.write_json_object(&root_payload)?;

        // 6. Write by-id index
        let by_id_index = serde_json::json!({
            "schema_version": GRAPH_STORE_SCHEMA_VERSION,
            "root_dir_id": graph.root_dir_id,
            "root_graph_hash": root_graph_hash,
            "root_object_hash": root_object_hash,
            "dirs": graph.dirs.iter().map(|d| &d.base.id).collect::<Vec<_>>(),
            "files": graph.files.iter().map(|f| &f.base.id).collect::<Vec<_>>(),
            "leaves": graph.leaves.iter().map(|l| &l.base.id).collect::<Vec<_>>(),
            "nodes": index_nodes,
        });
        write_json_file(&self.index_dir().join("by-id.json"), &by_id_index)?;

        // 7. Write refs/current.json
        let current_ref = serde_json::json!({
            "schema_version": GRAPH_STORE_SCHEMA_VERSION,
            "root_graph_hash": root_graph_hash,
            "root_object_hash": root_object_hash,
            "root_dir_id": graph.root_dir_id,
            "index": "graph/index/by-id.json",
        });
        write_json_file(&self.refs_dir().join("current.json"), &current_ref)?;

        Ok(root_graph_hash)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn ensure_dirs(&self) -> Result<(), KnowledgeError> {
        for dir in [
            self.refs_dir(),
            self.index_dir(),
            self.objects_dir(),
            self.blobs_dir(),
        ] {
            fs::create_dir_all(&dir).map_err(|e| {
                KnowledgeError::knowledge_unavailable(format!("create dir {}: {e}", dir.display()))
            })?;
        }
        Ok(())
    }

    fn read_blob(&self, blob_hash: &str) -> Result<String, KnowledgeError> {
        let path = self.blob_path(blob_hash);
        fs::read_to_string(&path).map_err(|e| {
            KnowledgeError::knowledge_unavailable(format!("read blob {}: {e}", path.display()))
        })
    }

    fn write_json_object(&self, payload: &Value) -> Result<String, KnowledgeError> {
        let canonical = canonical_json(payload);
        let digest = sha256_hex(canonical.as_bytes());
        let path = self.object_path(&digest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KnowledgeError::knowledge_unavailable(format!(
                    "create dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let sorted = sort_json_value(payload.clone());
        let pretty = serde_json::to_string_pretty(&sorted)
            .map_err(|e| KnowledgeError::invalid_data(format!("serialize object: {e}")))?;
        fs::write(&path, format!("{pretty}\n")).map_err(|e| {
            KnowledgeError::knowledge_unavailable(format!("write object {}: {e}", path.display()))
        })?;
        Ok(digest)
    }

    fn write_blob(&self, content: &str) -> Result<String, KnowledgeError> {
        let digest = sha256_hex(content.as_bytes());
        let path = self.blob_path(&digest);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                KnowledgeError::knowledge_unavailable(format!(
                    "create dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        fs::write(&path, content).map_err(|e| {
            KnowledgeError::knowledge_unavailable(format!("write blob {}: {e}", path.display()))
        })?;
        Ok(digest)
    }
}

// ---------------------------------------------------------------------------
// Free-standing file I/O helpers
// ---------------------------------------------------------------------------

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, KnowledgeError> {
    let content = fs::read_to_string(path).map_err(|e| {
        KnowledgeError::knowledge_unavailable(format!("read {}: {e}", path.display()))
    })?;
    serde_json::from_str(&content)
        .map_err(|e| KnowledgeError::invalid_data(format!("parse {}: {e}", path.display())))
}

fn write_json_file(path: &Path, payload: &Value) -> Result<(), KnowledgeError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            KnowledgeError::knowledge_unavailable(format!("create dir {}: {e}", parent.display()))
        })?;
    }
    let sorted = sort_json_value(payload.clone());
    let pretty = serde_json::to_string_pretty(&sorted)
        .map_err(|e| KnowledgeError::invalid_data(format!("serialize json: {e}")))?;
    fs::write(path, format!("{pretty}\n")).map_err(|e| {
        KnowledgeError::knowledge_unavailable(format!("write json {}: {e}", path.display()))
    })?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Canonical JSON (must match Python's json.dumps(sort_keys=True, separators=(",",":")))
// ---------------------------------------------------------------------------

fn canonical_json(value: &Value) -> String {
    let sorted = sort_json_value(value.clone());
    serde_json::to_string(&sorted).unwrap()
}

fn sort_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_json_value(v)))
                .collect();
            Value::Object(sorted.into_iter().collect())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sort_json_value).collect()),
        other => other,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::nodes::{BaseNodeFields, LeafKind};
    use super::*;

    fn fixture_graph_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../orbit-tools/tests/fixtures/knowledge/.orbit/knowledge/graph")
    }

    #[test]
    fn read_graph_from_fixture() {
        let store = GraphObjectStore::new(fixture_graph_dir());
        let graph = store.read_graph().unwrap();

        // Verify root
        assert_eq!(graph.root_dir_id, "node-dir-tools-src");

        // 1 dir, 1 file, 1 leaf
        assert_eq!(graph.dirs.len(), 1);
        assert_eq!(graph.files.len(), 1);
        assert_eq!(graph.leaves.len(), 1);

        // Dir node
        let dir = &graph.dirs[0];
        assert_eq!(dir.base.id, "node-dir-tools-src");
        assert_eq!(dir.base.location, "crates/orbit-tools/src");
        assert_eq!(
            dir.base.object_hash.as_deref(),
            Some("1111111111111111111111111111111111111111111111111111111111111111")
        );
        assert_eq!(dir.file_children, vec!["node-file-lib"]);

        // File node
        let file = &graph.files[0];
        assert_eq!(file.base.id, "node-file-lib");
        assert_eq!(file.base.location, "crates/orbit-tools/src/lib.rs");
        assert_eq!(
            file.base.object_hash.as_deref(),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert_eq!(file.leaf_children, vec!["node-leaf-register-builtins"]);

        // Leaf node -- source loaded from blob
        let leaf = &graph.leaves[0];
        assert_eq!(leaf.base.id, "node-leaf-register-builtins");
        assert_eq!(leaf.kind, LeafKind::Function);
        assert!(
            leaf.source.contains("register_builtins"),
            "source should be hydrated from blob"
        );
        assert_eq!(
            leaf.source_blob_hash.as_deref(),
            Some("5555555555555555555555555555555555555555555555555555555555555555")
        );
    }

    #[test]
    fn round_trip_write_then_read() {
        let tmp = tempfile::tempdir().unwrap();
        let graph_dir = tmp.path().join("graph");

        // Build a minimal graph
        let graph = CodebaseGraphV1 {
            root_dir_id: "dir-root".to_string(),
            dirs: vec![DirNode {
                base: BaseNodeFields {
                    id: "dir-root".to_string(),
                    identity_key: "dir:src".to_string(),
                    object_hash: None,
                    name: "src".to_string(),
                    location: "src".to_string(),
                    language: "rust".to_string(),
                    description: String::new(),
                    parent_id: None,
                    is_locked: false,
                    lineage_locked: false,
                    lock_owner: None,
                    lock_reason: String::new(),
                },
                dir_children: vec![],
                file_children: vec!["file-main".to_string()],
            }],
            files: vec![FileNode {
                base: BaseNodeFields {
                    id: "file-main".to_string(),
                    identity_key: "file:src/main.rs".to_string(),
                    object_hash: None,
                    name: "main.rs".to_string(),
                    location: "src/main.rs".to_string(),
                    language: "rust".to_string(),
                    description: String::new(),
                    parent_id: Some("dir-root".to_string()),
                    is_locked: false,
                    lineage_locked: false,
                    lock_owner: None,
                    lock_reason: String::new(),
                },
                extension: Some("rs".to_string()),
                source_blob_hash: None,
                imports: vec![],
                exports: vec![],
                leaf_children: vec!["leaf-main-fn".to_string()],
            }],
            leaves: vec![LeafNode {
                base: BaseNodeFields {
                    id: "leaf-main-fn".to_string(),
                    identity_key: "leaf:src/main.rs#main:function".to_string(),
                    object_hash: None,
                    name: "main".to_string(),
                    location: "src/main.rs#main".to_string(),
                    language: "rust".to_string(),
                    description: "Entry point.".to_string(),
                    parent_id: Some("file-main".to_string()),
                    is_locked: false,
                    lineage_locked: false,
                    lock_owner: None,
                    lock_reason: String::new(),
                },
                kind: LeafKind::Function,
                source: "fn main() { println!(\"hello\"); }".to_string(),
                source_blob_hash: None,
                source_hash: None,
                file_hash_at_capture: None,
                history: vec![],
                input_signature: vec![],
                output_signature: vec![],
                start_line: Some(1),
                end_line: Some(3),
                children: vec![],
            }],
        };

        let store = GraphObjectStore::new(&graph_dir);
        let root_hash = store.write_graph(&graph).unwrap();
        assert_eq!(root_hash.len(), 64, "root hash should be SHA-256 hex");

        // Read it back
        let loaded = store.read_graph().unwrap();

        assert_eq!(loaded.root_dir_id, graph.root_dir_id);
        assert_eq!(loaded.dirs.len(), 1);
        assert_eq!(loaded.files.len(), 1);
        assert_eq!(loaded.leaves.len(), 1);

        // Dir preserved
        assert_eq!(loaded.dirs[0].base.id, "dir-root");
        assert_eq!(loaded.dirs[0].file_children, vec!["file-main"]);

        // File preserved
        assert_eq!(loaded.files[0].base.id, "file-main");
        assert_eq!(loaded.files[0].leaf_children, vec!["leaf-main-fn"]);

        // Leaf preserved -- source round-trips through blob
        let leaf = &loaded.leaves[0];
        assert_eq!(leaf.base.id, "leaf-main-fn");
        assert_eq!(leaf.kind, LeafKind::Function);
        assert_eq!(leaf.source, "fn main() { println!(\"hello\"); }");
        assert!(
            leaf.source_blob_hash.is_some(),
            "blob hash should be set after write"
        );

        // Object hashes populated on read-back
        assert!(loaded.dirs[0].base.object_hash.is_some());
        assert!(loaded.files[0].base.object_hash.is_some());
        assert!(loaded.leaves[0].base.object_hash.is_some());
    }

    #[test]
    fn canonical_json_sorts_keys() {
        let val = serde_json::json!({"z": 1, "a": {"c": 3, "b": 2}});
        let result = canonical_json(&val);
        assert_eq!(result, r#"{"a":{"b":2,"c":3},"z":1}"#);
    }

    #[test]
    fn canonical_json_no_spaces() {
        let val = serde_json::json!({"key": [1, 2, 3]});
        let result = canonical_json(&val);
        // Must match Python: json.dumps(sort_keys=True, separators=(",",":"))
        assert_eq!(result, r#"{"key":[1,2,3]}"#);
    }
}
