use std::path::PathBuf;

use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use orbit_knowledge::pipeline::context::BuildConfig;

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Build and query the knowledge graph")]
pub struct KnowledgeCommand {
    #[command(subcommand)]
    pub subcommand: KnowledgeSubcommand,
}

#[derive(Subcommand)]
pub enum KnowledgeSubcommand {
    /// Build the knowledge graph from scratch
    Build(KnowledgeBuildArgs),
    /// Incrementally update the knowledge graph
    Update(KnowledgeUpdateArgs),
}

#[derive(Args)]
pub struct KnowledgeBuildArgs {
    /// Repository root (defaults to current working directory)
    #[arg(long)]
    pub repo: Option<PathBuf>,
}

#[derive(Args)]
pub struct KnowledgeUpdateArgs {
    /// Repository root (defaults to current working directory)
    #[arg(long)]
    pub repo: Option<PathBuf>,
}

impl Execute for KnowledgeCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.subcommand {
            KnowledgeSubcommand::Build(args) => args.execute(runtime),
            KnowledgeSubcommand::Update(args) => args.execute(runtime),
        }
    }
}

impl Execute for KnowledgeBuildArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        run_pipeline(runtime, self.repo, false)
    }
}

impl Execute for KnowledgeUpdateArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        run_pipeline(runtime, self.repo, true)
    }
}

fn run_pipeline(
    runtime: &OrbitRuntime,
    repo_override: Option<PathBuf>,
    incremental: bool,
) -> Result<(), OrbitError> {
    let data_root = runtime.data_root();
    let repo_path = repo_override.unwrap_or_else(|| {
        data_root
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    let output_dir = data_root.join("knowledge");

    let mode = if incremental { "update" } else { "build" };
    eprintln!("knowledge {mode}: scanning {}", repo_path.display());

    let config = BuildConfig {
        repo_path: repo_path.clone(),
        output_dir,
        incremental,
    };

    let ctx = orbit_knowledge::pipeline::run_build(config)
        .map_err(|e| OrbitError::Execution(format!("knowledge {mode} failed: {e}")))?;

    eprintln!(
        "knowledge {mode}: {} dirs, {} files, {} leaves",
        ctx.graph.dirs.len(),
        ctx.graph.files.len(),
        ctx.graph.leaves.len(),
    );
    eprintln!("knowledge {mode}: written to {}", ctx.output_dir.display());

    Ok(())
}
