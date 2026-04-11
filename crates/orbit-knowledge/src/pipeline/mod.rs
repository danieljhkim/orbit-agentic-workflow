//! Build pipeline: scan → hash → build graph → persist.
//!
//! Each stage is a plain function operating on a shared [`PipelineContext`].

pub mod build;
pub mod context;
pub mod hash;
pub mod persist;
pub mod scan;

use crate::error::KnowledgeError;
use context::{BuildConfig, PipelineContext};

/// Run the full build pipeline.
///
/// Scans the repo, computes hashes, builds the graph (dirs, files, leaves),
/// persists the graph to the content-addressed store, and writes the manifest.
pub fn run_build(config: BuildConfig) -> Result<PipelineContext, KnowledgeError> {
    let mut ctx = PipelineContext::new(config);

    scan::scan_repo(&mut ctx)?;
    hash::compute_hashes(&mut ctx)?;
    hash::detect_changes(&mut ctx)?;

    build::build_graph_dirs(&mut ctx)?;
    build::build_graph_files(&mut ctx)?;
    build::build_graph_leaves(&mut ctx)?;

    persist::persist_graph(&ctx)?;
    persist::write_manifest(&ctx)?;
    hash::save_hash_cache(&ctx)?;

    Ok(ctx)
}
