pub mod activity;
pub mod audit;
pub mod config;
pub mod duel;
pub mod graph;
pub mod init;
pub mod job;
pub(crate) mod job_run_support;
pub mod metrics;
pub mod ship;
pub mod skill;
pub mod task;
pub mod tool;
pub mod workspace;

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

pub trait Execute {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError>;
}

#[derive(Parser)]
#[command(name = "orbit")]
#[command(about = "Orbit CLI", version)]
#[command(
    disable_help_subcommand = true,
    help_template = "\
{name} {version}

{usage-heading} {usage}

Setup:
  init       Initialize the global Orbit root (~/.orbit)
  workspace  Initialize and manage workspaces
  config     Show or update Orbit configuration

Run workflows:
  ship       Ship tasks through the pipeline
  duel       Cross-agent scoring

Manage work:
  task       Create, update, and manage tasks
  tool       Manage and run Orbit tools
  skill      Manage agent skill definitions
  graph      Build and query the knowledge graph

Inspect:
  audit      Query the audit event log
  metrics    Inspect token, tool-call, and knowledge-pack metrics

Options:
{options}"
)]
pub struct Cli {
    /// Override the Orbit root directory (highest precedence)
    #[arg(long, global = true)]
    pub root: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // ── setup ──
    Init(init::InitCommand),
    Workspace(workspace::WorkspaceCommand),
    Config(config::ConfigCommand),

    // ── run workflows ──
    Ship(ship::ShipCommand),
    Duel(duel::DuelCommand),

    // ── manage work ──
    Task(task::TaskCommand),
    Activity(activity::ActivityCommand),
    Job(job::JobCommand),
    Tool(tool::ToolCommand),
    Skill(skill::SkillCommand),
    Graph(graph::GraphCommand),

    // ── inspect ──
    Audit(audit::AuditCommand),
    Metrics(metrics::MetricsCommand),
}

impl Execute for Commands {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            Commands::Init(cmd) => cmd.execute(runtime),
            Commands::Workspace(cmd) => cmd.execute(runtime),
            Commands::Config(cmd) => cmd.execute(runtime),
            Commands::Ship(cmd) => cmd.execute(runtime),
            Commands::Duel(cmd) => cmd.execute(runtime),
            Commands::Task(cmd) => cmd.execute(runtime),
            Commands::Activity(cmd) => cmd.execute(runtime),
            Commands::Job(cmd) => cmd.execute(runtime),
            Commands::Tool(cmd) => cmd.execute(runtime),
            Commands::Skill(cmd) => cmd.execute(runtime),
            Commands::Graph(cmd) => cmd.execute(runtime),
            Commands::Audit(cmd) => cmd.execute(runtime),
            Commands::Metrics(cmd) => cmd.execute(runtime),
        }
    }
}
