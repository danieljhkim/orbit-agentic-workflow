use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// LeafKind
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LeafKind {
    Function,
    Method,
    Class,
    Struct,
    Interface,
    Trait,
    Impl,
    Field,
    Module,
}

impl fmt::Display for LeafKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Field => "field",
            Self::Module => "module",
        };
        f.write_str(s)
    }
}

// ---------------------------------------------------------------------------
// SignatureField
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureField {
    pub name: String,
    #[serde(default)]
    pub annotation: Option<String>,
    #[serde(default)]
    pub description: String,
}

// ---------------------------------------------------------------------------
// LeafHistoryEntry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafHistoryEntry {
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub source: String,
    pub source_blob_hash: Option<String>,
    pub source_hash: Option<String>,
    pub file_hash_at_capture: Option<String>,
}

// ---------------------------------------------------------------------------
// BaseNodeFields
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaseNodeFields {
    pub id: String,
    #[serde(default)]
    pub identity_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_hash: Option<String>,
    pub name: String,
    pub location: String,
    pub language: String,
    #[serde(default)]
    pub description: String,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default)]
    pub lineage_locked: bool,
    pub lock_owner: Option<String>,
    #[serde(default)]
    pub lock_reason: String,
}

// ---------------------------------------------------------------------------
// DirNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirNode {
    #[serde(flatten)]
    pub base: BaseNodeFields,
    #[serde(default)]
    pub dir_children: Vec<String>,
    #[serde(default)]
    pub file_children: Vec<String>,
}

// ---------------------------------------------------------------------------
// FileNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileNode {
    #[serde(flatten)]
    pub base: BaseNodeFields,
    pub extension: Option<String>,
    pub source_blob_hash: Option<String>,
    #[serde(default)]
    pub imports: Vec<String>,
    #[serde(default)]
    pub exports: Vec<String>,
    #[serde(default)]
    pub leaf_children: Vec<String>,
}

// ---------------------------------------------------------------------------
// LeafNode
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafNode {
    #[serde(flatten)]
    pub base: BaseNodeFields,
    pub kind: LeafKind,
    #[serde(default)]
    pub source: String,
    pub source_blob_hash: Option<String>,
    pub source_hash: Option<String>,
    pub file_hash_at_capture: Option<String>,
    #[serde(default)]
    pub history: Vec<LeafHistoryEntry>,
    #[serde(default)]
    pub input_signature: Vec<SignatureField>,
    #[serde(default)]
    pub output_signature: Vec<SignatureField>,
    pub start_line: Option<u32>,
    pub end_line: Option<u32>,
    #[serde(default)]
    pub children: Vec<String>,
}

// ---------------------------------------------------------------------------
// GraphNode (untagged wrapper — constructed programmatically, not via serde)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum GraphNode {
    Dir(DirNode),
    File(FileNode),
    Leaf(LeafNode),
}

impl GraphNode {
    pub fn id(&self) -> &str {
        &self.base().id
    }

    pub fn base(&self) -> &BaseNodeFields {
        match self {
            Self::Dir(n) => &n.base,
            Self::File(n) => &n.base,
            Self::Leaf(n) => &n.base,
        }
    }

    pub fn location(&self) -> &str {
        &self.base().location
    }

    pub fn parent_id(&self) -> Option<&str> {
        self.base().parent_id.as_deref()
    }
}

// ---------------------------------------------------------------------------
// CodebaseGraphV1
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseGraphV1 {
    pub root_dir_id: String,
    #[serde(default)]
    pub dirs: Vec<DirNode>,
    #[serde(default)]
    pub files: Vec<FileNode>,
    #[serde(default)]
    pub leaves: Vec<LeafNode>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_leaf_node_from_fixture() {
        let fixture = include_str!(
            "../../../orbit-tools/tests/fixtures/knowledge/.orbit/knowledge/graph/objects/33/\
             3333333333333333333333333333333333333333333333333333333333333333.json"
        );

        let wrapper: serde_json::Value =
            serde_json::from_str(fixture).expect("fixture should be valid JSON");

        let node_value = wrapper
            .get("node")
            .expect("fixture should have a \"node\" field");

        let leaf: LeafNode = serde_json::from_value(node_value.clone())
            .expect("node should deserialize as LeafNode");

        assert_eq!(leaf.base.id, "node-leaf-register-builtins");
        assert_eq!(leaf.kind, LeafKind::Function);
        assert_eq!(leaf.start_line, Some(11));
        assert_eq!(leaf.end_line, Some(22));
        assert_eq!(
            leaf.base.location,
            "crates/orbit-tools/src/lib.rs#register_builtins"
        );
        assert_eq!(
            leaf.source_blob_hash.as_deref(),
            Some("5555555555555555555555555555555555555555555555555555555555555555")
        );
    }

    #[test]
    fn leaf_kind_display_lowercase() {
        assert_eq!(LeafKind::Function.to_string(), "function");
        assert_eq!(LeafKind::Trait.to_string(), "trait");
        assert_eq!(LeafKind::Impl.to_string(), "impl");
    }

    #[test]
    fn graph_node_helpers() {
        let base = BaseNodeFields {
            id: "n1".into(),
            identity_key: String::new(),
            object_hash: None,
            name: "root".into(),
            location: "/".into(),
            language: "rust".into(),
            description: String::new(),
            parent_id: None,
            is_locked: false,
            lineage_locked: false,
            lock_owner: None,
            lock_reason: String::new(),
        };
        let dir = GraphNode::Dir(DirNode {
            base,
            dir_children: vec![],
            file_children: vec![],
        });
        assert_eq!(dir.id(), "n1");
        assert_eq!(dir.location(), "/");
        assert_eq!(dir.parent_id(), None);
    }

    #[test]
    fn object_hash_skipped_when_none() {
        let base = BaseNodeFields {
            id: "x".into(),
            identity_key: String::new(),
            object_hash: None,
            name: "n".into(),
            location: "l".into(),
            language: "rs".into(),
            description: String::new(),
            parent_id: None,
            is_locked: false,
            lineage_locked: false,
            lock_owner: None,
            lock_reason: String::new(),
        };
        let json = serde_json::to_value(&base).unwrap();
        assert!(!json.as_object().unwrap().contains_key("object_hash"));
        // parent_id should still be present as null
        assert!(json.as_object().unwrap().contains_key("parent_id"));
        assert!(json["parent_id"].is_null());
    }
}
