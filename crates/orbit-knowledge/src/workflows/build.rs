//! Knowledge-graph build / incremental-update workflow.
//!
//! Consumed by `orbit-core::command::init` (workspace bootstrap) and the
//! `orbit observe graph build|update` CLI subcommands. Returns
//! [`KnowledgeError`]; host crates translate to `OrbitError` at the edge.

use std::path::PathBuf;

use crate::KnowledgeError;
use crate::graph::object_store::RefName;
use crate::pipeline::context::BuildConfig;
use crate::workflows::{parse_ref_name, repo_from_data_root};

#[derive(Debug, Clone)]
pub struct GraphBuildOptions {
    pub data_root: PathBuf,
    pub repo_override: Option<PathBuf>,
    pub ref_name: Option<String>,
    pub incremental: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedGraphBuild {
    pub mode: &'static str,
    pub repo_path: PathBuf,
    pub output_dir: PathBuf,
    incremental: bool,
    ref_name: Option<RefName>,
}

#[derive(Debug, Clone)]
pub struct GraphBuildOutput {
    pub mode: &'static str,
    pub output_dir: PathBuf,
    pub dirs: usize,
    pub files: usize,
    pub leaves: usize,
}

pub fn resolve_graph_build(
    options: GraphBuildOptions,
) -> Result<ResolvedGraphBuild, KnowledgeError> {
    let repo_path = options
        .repo_override
        .unwrap_or_else(|| repo_from_data_root(&options.data_root));
    let output_dir = options.data_root.join("knowledge");
    let mode = if options.incremental {
        "update"
    } else {
        "build"
    };
    Ok(ResolvedGraphBuild {
        mode,
        repo_path,
        output_dir,
        incremental: options.incremental,
        ref_name: parse_ref_name(options.ref_name)?,
    })
}

pub fn run_resolved_graph_build(
    resolved: ResolvedGraphBuild,
) -> Result<GraphBuildOutput, KnowledgeError> {
    let config = BuildConfig {
        repo_path: resolved.repo_path,
        output_dir: resolved.output_dir.clone(),
        incremental: resolved.incremental,
        ref_name: resolved.ref_name,
    };

    let ctx = crate::pipeline::run_build(config).map_err(|error| {
        KnowledgeError::knowledge_unavailable(format!(
            "knowledge {} failed: {error}",
            resolved.mode
        ))
    })?;

    Ok(GraphBuildOutput {
        mode: resolved.mode,
        output_dir: ctx.output_dir,
        dirs: ctx.graph.dirs.len(),
        files: ctx.graph.files.len(),
        leaves: ctx.graph.leaves.len(),
    })
}

pub fn build_graph(options: GraphBuildOptions) -> Result<GraphBuildOutput, KnowledgeError> {
    run_resolved_graph_build(resolve_graph_build(options)?)
}
