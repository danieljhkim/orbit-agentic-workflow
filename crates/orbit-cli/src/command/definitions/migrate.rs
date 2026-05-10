use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::json;

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Run one-shot Orbit data migrations")]
pub struct MigrateCommand {
    #[command(subcommand)]
    pub command: MigrateSubcommand,
}

impl Execute for MigrateCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum MigrateSubcommand {
    /// Convert legacy friction tasks into .orbit/frictions records
    Frictions(MigrateFrictionsArgs),
    /// Rewrite legacy task type values to the current taxonomy
    TaskTypes(MigrateTaskTypesArgs),
}

impl Execute for MigrateSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            MigrateSubcommand::Frictions(args) => args.execute(runtime),
            MigrateSubcommand::TaskTypes(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct MigrateFrictionsArgs {
    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MigrateFrictionsArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let summary = runtime.migrate_legacy_frictions()?;
        if self.json {
            return crate::output::json::print_pretty(&json!({
                "created": summary.created,
                "skipped": summary.skipped,
            }));
        }
        println!(
            "migrated legacy frictions: {} created, {} skipped",
            summary.created, summary.skipped
        );
        Ok(())
    }
}

#[derive(Args)]
pub struct MigrateTaskTypesArgs {
    /// Print proposed changes without writing task files
    #[arg(long)]
    pub dry_run: bool,
    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for MigrateTaskTypesArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let summary = runtime.migrate_task_types(self.dry_run)?;
        if self.json {
            return crate::output::json::print_pretty(&json!({
                "dry_run": self.dry_run,
                "changed": summary.changed,
                "unchanged": summary.unchanged,
                "friction_skipped": summary.friction_skipped,
                "changes": summary.changes.iter().map(|change| {
                    json!({
                        "task_id": change.task_id.as_deref(),
                        "path": change.path.display().to_string(),
                        "from": change.from.as_str(),
                        "to": change.to.as_str(),
                    })
                }).collect::<Vec<_>>(),
            }));
        }

        for change in &summary.changes {
            let task = change
                .task_id
                .as_deref()
                .unwrap_or_else(|| change.path.to_str().unwrap_or("<non-utf8 path>"));
            println!("{task}: {} -> {}", change.from, change.to);
        }
        let verb = if self.dry_run {
            "would migrate"
        } else {
            "migrated"
        };
        println!(
            "{verb} task types: {} changed, {} unchanged, {} friction skipped",
            summary.changed, summary.unchanged, summary.friction_skipped
        );
        Ok(())
    }
}
