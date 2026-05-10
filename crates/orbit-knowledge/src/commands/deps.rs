use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::KnowledgeError;
use crate::service::deps::crate_dependencies;

#[derive(Debug, Clone)]
pub struct DepsInput {
    pub workspace_root: PathBuf,
    pub crate_filter: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DepsResult {
    pub workspace: PathBuf,
    pub crates: BTreeMap<String, Vec<String>>,
}

pub fn run(input: DepsInput) -> Result<DepsResult, KnowledgeError> {
    let crates = crate_dependencies(&input.workspace_root, input.crate_filter.as_deref())?;
    Ok(DepsResult {
        workspace: input.workspace_root,
        crates,
    })
}
