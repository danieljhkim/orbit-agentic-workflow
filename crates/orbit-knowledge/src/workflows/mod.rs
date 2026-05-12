//! Application-level workflows over the knowledge graph.
//!
//! Entry points for crates that *host* the knowledge graph (orbit-core's
//! workspace bootstrap, orbit-cli's `observe graph` subcommands) rather
//! than fine-grained agent tool commands (see `crate::commands` for
//! those). Each submodule composes [`crate::pipeline`], [`crate::service`],
//! and [`crate::graph`] into a single coarse-grained operation.
//!
//! All workflow entry points return [`KnowledgeError`]; host crates
//! translate at the edge with [`crate::knowledge_error_to_orbit`].

use std::path::{Path, PathBuf};

use crate::KnowledgeError;
use crate::commands::knowledge_error_from_orbit;
use crate::graph::nodes::CodebaseGraphV1;
use crate::graph::object_store::RefName;
use crate::{GraphReadOptions, TaskGraphScope, TaskGraphService};

pub mod build;
pub mod observe;

pub(crate) fn load_graph(
    data_root: &Path,
    explicit_ref: Option<&str>,
    options: GraphReadOptions,
) -> Result<CodebaseGraphV1, KnowledgeError> {
    let knowledge_dir = data_root.join("knowledge");
    let repo_path = repo_from_data_root(data_root);
    let service = TaskGraphService::new(knowledge_dir, TaskGraphScope::default());
    service
        .read_graph(Some(&repo_path), false, explicit_ref, options)
        .map_err(knowledge_error_from_orbit)
}

pub(crate) fn repo_from_data_root(data_root: &Path) -> PathBuf {
    data_root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn parse_ref_name(ref_name: Option<String>) -> Result<Option<RefName>, KnowledgeError> {
    ref_name
        .filter(|value| !value.trim().is_empty())
        .map(RefName::new)
        .transpose()
        .map_err(|error| KnowledgeError::invalid_data(error.to_string()))
}
