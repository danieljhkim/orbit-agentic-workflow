use std::path::PathBuf;

use clap::{Args, Subcommand};
use orbit_core::command::adr_migration::{MigrationOptions, run_migration};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Architecture Decision Record (ADR) operations")]
pub struct AdrCommand {
    #[command(subcommand)]
    pub command: AdrSubcommand,
}

#[derive(Subcommand)]
pub enum AdrSubcommand {
    /// One-shot migrate `docs/design/*/4_decisions.md` corpus into the v2 store
    Migrate(AdrMigrateArgs),
}

#[derive(Args)]
pub struct AdrMigrateArgs {
    /// Walk the corpus and emit `migration-report.md` without writing artifacts
    /// or rewriting source files.
    #[arg(long)]
    pub dry_run: bool,
    /// Override the workspace root containing `docs/design/`. Defaults to the
    /// runtime's repo root.
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

impl Execute for AdrCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.command {
            AdrSubcommand::Migrate(args) => migrate(runtime, args),
        }
    }
}

fn migrate(runtime: &OrbitRuntime, args: AdrMigrateArgs) -> Result<(), OrbitError> {
    let options = MigrationOptions {
        workspace_path: args.workspace,
        dry_run: args.dry_run,
    };
    let report = run_migration(runtime, options)?;
    println!(
        "ADR migration {}: created={}, skipped={}, validation_warnings={}, rollups={}, rewrites={}, unresolved={}",
        if args.dry_run {
            "(dry-run)"
        } else {
            "complete"
        },
        report.created.len(),
        report.skipped.len(),
        report.validation_warnings.len(),
        report.rollups.len(),
        report.rewrites.len(),
        report.unresolved_references.len(),
    );
    println!("see migration-report.md for the full breakdown");
    Ok(())
}
