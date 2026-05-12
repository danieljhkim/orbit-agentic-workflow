//! Thin translation layer over the knowledge-graph workflows in
//! `orbit_knowledge::workflows::{build, observe}`.
//!
//! Callers in this workspace (`orbit-core::command::init`, the
//! `orbit observe graph` CLI subcommands) want `Result<_, OrbitError>`,
//! so wrap each entry point in [`orbit_knowledge::knowledge_error_to_orbit`].
//! Adding a new graph workflow lives in `orbit_knowledge::workflows::*`;
//! mirror it here only if a non-knowledge consumer needs the `OrbitError`
//! surface.

use orbit_common::types::OrbitError;
use orbit_knowledge::knowledge_error_to_orbit;

pub use orbit_knowledge::default_orbitignore_template;
pub use orbit_knowledge::workflows::build::{
    GraphBuildOptions, GraphBuildOutput, ResolvedGraphBuild,
};
pub use orbit_knowledge::workflows::observe::{
    GraphHistoryOptions, GraphNodeDetails, GraphSearchOptions, GraphSearchOutput, GraphShowOptions,
    GraphShowOutput,
};

pub fn resolve_graph_build(options: GraphBuildOptions) -> Result<ResolvedGraphBuild, OrbitError> {
    orbit_knowledge::workflows::build::resolve_graph_build(options)
        .map_err(knowledge_error_to_orbit)
}

pub fn run_resolved_graph_build(
    resolved: ResolvedGraphBuild,
) -> Result<GraphBuildOutput, OrbitError> {
    orbit_knowledge::workflows::build::run_resolved_graph_build(resolved)
        .map_err(knowledge_error_to_orbit)
}

pub fn build_graph(options: GraphBuildOptions) -> Result<GraphBuildOutput, OrbitError> {
    orbit_knowledge::workflows::build::build_graph(options).map_err(knowledge_error_to_orbit)
}

pub fn show_graph(options: GraphShowOptions) -> Result<GraphShowOutput, OrbitError> {
    orbit_knowledge::workflows::observe::show_graph(options).map_err(knowledge_error_to_orbit)
}

pub fn search_graph(options: GraphSearchOptions) -> Result<GraphSearchOutput, OrbitError> {
    orbit_knowledge::workflows::observe::search_graph(options).map_err(knowledge_error_to_orbit)
}

pub fn history_graph(options: GraphHistoryOptions) -> Result<(), OrbitError> {
    orbit_knowledge::workflows::observe::history_graph(options).map_err(knowledge_error_to_orbit)
}
