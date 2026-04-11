use std::collections::HashMap;

use crate::error::KnowledgeError;

use super::nodes::{BaseNodeFields, CodebaseGraphV1, DirNode, FileNode, LeafNode};

/// Borrowed reference to a graph node.
#[derive(Debug, Clone, Copy)]
pub enum GraphNodeRef<'a> {
    Dir(&'a DirNode),
    File(&'a FileNode),
    Leaf(&'a LeafNode),
}

impl<'a> GraphNodeRef<'a> {
    pub fn id(&self) -> &str {
        &self.base().id
    }

    pub fn base(&self) -> &BaseNodeFields {
        match self {
            GraphNodeRef::Dir(n) => &n.base,
            GraphNodeRef::File(n) => &n.base,
            GraphNodeRef::Leaf(n) => &n.base,
        }
    }

    pub fn parent_id(&self) -> Option<&str> {
        self.base().parent_id.as_deref()
    }

    pub fn location(&self) -> &str {
        &self.base().location
    }

    pub fn child_ids(&self) -> Vec<&str> {
        match self {
            GraphNodeRef::Dir(n) => n
                .dir_children
                .iter()
                .chain(n.file_children.iter())
                .map(String::as_str)
                .collect(),
            GraphNodeRef::File(n) => n.leaf_children.iter().map(String::as_str).collect(),
            GraphNodeRef::Leaf(n) => n.children.iter().map(String::as_str).collect(),
        }
    }
}

/// Provides traversal methods over a [`CodebaseGraphV1`].
///
/// Builds an internal index on construction for O(1) lookups by node ID.
pub struct GraphNavigator<'a> {
    graph: &'a CodebaseGraphV1,
    node_index: HashMap<&'a str, GraphNodeRef<'a>>,
}

impl<'a> GraphNavigator<'a> {
    pub fn new(graph: &'a CodebaseGraphV1) -> Self {
        let mut node_index = HashMap::new();
        for dir in &graph.dirs {
            node_index.insert(dir.base.id.as_str(), GraphNodeRef::Dir(dir));
        }
        for file in &graph.files {
            node_index.insert(file.base.id.as_str(), GraphNodeRef::File(file));
        }
        for leaf in &graph.leaves {
            node_index.insert(leaf.base.id.as_str(), GraphNodeRef::Leaf(leaf));
        }
        Self { graph, node_index }
    }

    pub fn get_node(&self, id: &str) -> Result<GraphNodeRef<'a>, KnowledgeError> {
        self.node_index
            .get(id)
            .copied()
            .ok_or_else(|| KnowledgeError::invalid_data(format!("node not found: {id}")))
    }

    pub fn get_root(&self) -> Result<GraphNodeRef<'a>, KnowledgeError> {
        self.get_node(&self.graph.root_dir_id)
    }

    pub fn get_parent(&self, id: &str) -> Result<Option<GraphNodeRef<'a>>, KnowledgeError> {
        let node = self.get_node(id)?;
        match node.parent_id() {
            Some(pid) => Ok(Some(self.get_node(pid)?)),
            None => Ok(None),
        }
    }

    pub fn get_children(&self, id: &str) -> Result<Vec<GraphNodeRef<'a>>, KnowledgeError> {
        let node = self.get_node(id)?;
        node.child_ids()
            .into_iter()
            .map(|cid| self.get_node(cid))
            .collect()
    }

    pub fn get_siblings(&self, id: &str) -> Result<Vec<GraphNodeRef<'a>>, KnowledgeError> {
        let node = self.get_node(id)?;
        let parent_id = match node.parent_id() {
            Some(pid) => pid,
            None => return Ok(vec![]),
        };
        let parent = self.get_node(parent_id)?;
        Ok(parent
            .child_ids()
            .into_iter()
            .filter_map(|cid| {
                if cid == id {
                    None
                } else {
                    self.node_index.get(cid).copied()
                }
            })
            .collect())
    }

    /// Returns the lineage (ancestor chain) for a node, in root-first order.
    pub fn get_lineage(
        &self,
        id: &str,
        include_self: bool,
    ) -> Result<Vec<GraphNodeRef<'a>>, KnowledgeError> {
        let node = self.get_node(id)?;
        let mut chain = Vec::new();
        let mut current = node;
        while let Some(pid) = current.parent_id() {
            let parent = self.get_node(pid)?;
            chain.push(parent);
            current = parent;
        }
        chain.reverse();
        if include_self {
            chain.push(node);
        }
        Ok(chain)
    }

    /// For a leaf, walk up ancestors to find the containing file node.
    pub fn get_containing_file(&self, id: &str) -> Result<Option<&'a FileNode>, KnowledgeError> {
        let mut current = self.get_node(id)?;
        loop {
            match current {
                GraphNodeRef::File(f) => return Ok(Some(f)),
                _ => match current.parent_id() {
                    Some(pid) => current = self.get_node(pid)?,
                    None => return Ok(None),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_base(id: &str, parent_id: Option<&str>) -> BaseNodeFields {
        BaseNodeFields {
            id: id.to_string(),
            identity_key: String::new(),
            object_hash: None,
            name: id.to_string(),
            location: id.to_string(),
            language: "rust".to_string(),
            description: String::new(),
            parent_id: parent_id.map(ToString::to_string),
            is_locked: false,
            lineage_locked: false,
            lock_owner: None,
            lock_reason: String::new(),
        }
    }

    fn fixture_graph() -> CodebaseGraphV1 {
        CodebaseGraphV1 {
            root_dir_id: "dir-root".to_string(),
            dirs: vec![DirNode {
                base: make_base("dir-root", None),
                dir_children: vec![],
                file_children: vec!["file-main".to_string()],
            }],
            files: vec![FileNode {
                base: make_base("file-main", Some("dir-root")),
                extension: Some("rs".to_string()),
                source_blob_hash: None,
                imports: vec![],
                exports: vec![],
                leaf_children: vec!["leaf-fn".to_string()],
            }],
            leaves: vec![LeafNode {
                base: make_base("leaf-fn", Some("file-main")),
                kind: super::super::nodes::LeafKind::Function,
                source: String::new(),
                source_blob_hash: None,
                source_hash: None,
                file_hash_at_capture: None,
                history: vec![],
                input_signature: vec![],
                output_signature: vec![],
                start_line: Some(1),
                end_line: Some(10),
                children: vec![],
            }],
        }
    }

    #[test]
    fn get_node_returns_correct_type() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        assert!(matches!(
            nav.get_node("dir-root").unwrap(),
            GraphNodeRef::Dir(_)
        ));
        assert!(matches!(
            nav.get_node("file-main").unwrap(),
            GraphNodeRef::File(_)
        ));
        assert!(matches!(
            nav.get_node("leaf-fn").unwrap(),
            GraphNodeRef::Leaf(_)
        ));
    }

    #[test]
    fn get_node_not_found() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        assert!(nav.get_node("nonexistent").is_err());
    }

    #[test]
    fn get_root() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let root = nav.get_root().unwrap();
        assert_eq!(root.id(), "dir-root");
    }

    #[test]
    fn get_parent_leaf_returns_file() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let parent = nav.get_parent("leaf-fn").unwrap().unwrap();
        assert_eq!(parent.id(), "file-main");
    }

    #[test]
    fn get_parent_root_returns_none() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        assert!(nav.get_parent("dir-root").unwrap().is_none());
    }

    #[test]
    fn get_children_dir() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let children = nav.get_children("dir-root").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "file-main");
    }

    #[test]
    fn get_children_file() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let children = nav.get_children("file-main").unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id(), "leaf-fn");
    }

    #[test]
    fn get_lineage_without_self() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let lineage = nav.get_lineage("leaf-fn", false).unwrap();
        let ids: Vec<&str> = lineage.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec!["dir-root", "file-main"]);
    }

    #[test]
    fn get_lineage_with_self() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let lineage = nav.get_lineage("leaf-fn", true).unwrap();
        let ids: Vec<&str> = lineage.iter().map(|n| n.id()).collect();
        assert_eq!(ids, vec!["dir-root", "file-main", "leaf-fn"]);
    }

    #[test]
    fn get_containing_file() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let file = nav.get_containing_file("leaf-fn").unwrap().unwrap();
        assert_eq!(file.base.id, "file-main");
    }

    #[test]
    fn get_siblings_only_child() {
        let graph = fixture_graph();
        let nav = GraphNavigator::new(&graph);
        let siblings = nav.get_siblings("leaf-fn").unwrap();
        assert!(siblings.is_empty());
    }
}
