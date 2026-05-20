use clap::{Args, Subcommand};
use orbit_core::command::semantic::{
    SemanticInstallParams, SemanticReindexParams, SemanticUninstallParams,
};
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::json;

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Manage local orbit-search indexing")]
pub struct SemanticCommand {
    #[command(subcommand)]
    pub command: SemanticSubcommand,
}

#[derive(Subcommand)]
pub enum SemanticSubcommand {
    /// Download the search companion and selected model
    Install(SemanticInstallArgs),
    /// Remove installed orbit-search companion and/or models
    Uninstall(SemanticUninstallArgs),
    /// Show orbit-search index and companion status
    Stats(SemanticStatsArgs),
    /// Rebuild task embeddings
    Index(SemanticIndexArgs),
}

#[derive(Args)]
pub struct SemanticInstallArgs {
    #[arg(long)]
    pub model: Option<String>,
    /// Replace the companion even when the installed version is current
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SemanticUninstallArgs {
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub all: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SemanticIndexArgs {
    #[arg(long)]
    pub model: Option<String>,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct SemanticStatsArgs {
    #[arg(long)]
    pub json: bool,
}

impl Execute for SemanticCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

impl Execute for SemanticSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            SemanticSubcommand::Install(args) => args.execute(runtime),
            SemanticSubcommand::Uninstall(args) => args.execute(runtime),
            SemanticSubcommand::Stats(args) => args.execute(runtime),
            SemanticSubcommand::Index(args) => args.execute(runtime),
        }
    }
}

impl Execute for SemanticInstallArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.semantic_install(SemanticInstallParams {
            model: self.model,
            force: self.force,
        })?;
        if self.json {
            crate::output::json::print_pretty(&json!(result))
        } else {
            println!(
                "Installed semantic search: companion={} model={} companion_changed={} model_changed={}",
                result.companion_path,
                result.model_id,
                result.companion_changed,
                result.model_installed
            );
            Ok(())
        }
    }
}

impl Execute for SemanticUninstallArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.semantic_uninstall(SemanticUninstallParams {
            model: self.model,
            all: self.all,
        })?;
        if self.json {
            crate::output::json::print_pretty(&json!(result))
        } else {
            println!(
                "Removed semantic search assets: companion={} models={}",
                result.removed_companion,
                if result.removed_models.is_empty() {
                    "-".to_string()
                } else {
                    result.removed_models.join(", ")
                }
            );
            Ok(())
        }
    }
}

impl Execute for SemanticIndexArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.semantic_reindex(SemanticReindexParams {
            model: self.model,
            force: self.force,
        })?;
        if self.json {
            crate::output::json::print_pretty(&json!(result))
        } else {
            println!(
                "Indexed semantic search: model={} embedded_chunks={} skipped_fields={}",
                result.model_id, result.report.embedded_chunks, result.report.skipped_fields
            );
            Ok(())
        }
    }
}

impl Execute for SemanticStatsArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.semantic_stats()?;
        if self.json {
            crate::output::json::print_pretty(&json!(result))
        } else {
            let mut table = crate::output::table::build_table(&["SOURCE_KIND", "MODEL", "ROWS"]);
            for row in &result.rows.counts {
                table.add_row(vec![
                    row.source_kind.clone(),
                    row.model_id.clone(),
                    row.rows.to_string(),
                ]);
            }
            println!("{table}");
            println!(
                "stale_rows={} companion={} version={} active_model={}",
                result.rows.stale_rows,
                if result.companion.installed {
                    "installed"
                } else {
                    "not_installed"
                },
                result.companion.version.as_deref().unwrap_or("-"),
                result.companion.model.as_deref().unwrap_or("-")
            );
            Ok(())
        }
    }
}
