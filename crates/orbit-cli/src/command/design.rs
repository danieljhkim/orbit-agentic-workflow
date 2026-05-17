use std::path::PathBuf;

use clap::{Args, Subcommand};
use orbit_core::command::design::{
    DesignDecayStatus, DesignFeatureSummary, init_feature, list_features, show_feature,
};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Design doc operations")]
pub struct DesignCommand {
    #[command(subcommand)]
    pub command: DesignSubcommand,
}

#[derive(Subcommand)]
pub enum DesignSubcommand {
    /// Scaffold a design-doc feature folder
    Init(DesignInitArgs),
    /// List design-doc feature folders
    List(DesignListArgs),
    /// Show one design-doc feature folder
    Show(DesignShowArgs),
}

#[derive(Args)]
pub struct DesignInitArgs {
    /// Lowercase, hyphenated feature folder name
    pub feature: String,
    /// Owner written into scaffolded frontmatter
    #[arg(long)]
    pub owner: Option<String>,
    /// Override the workspace root containing docs/design/
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Args)]
pub struct DesignListArgs {
    /// Override the workspace root containing docs/design/
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

#[derive(Args)]
pub struct DesignShowArgs {
    /// Lowercase, hyphenated feature folder name
    pub feature: String,
    /// Override the workspace root containing docs/design/
    #[arg(long)]
    pub workspace: Option<PathBuf>,
}

impl Execute for DesignCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.command {
            DesignSubcommand::Init(args) => init(runtime, args),
            DesignSubcommand::List(args) => list(runtime, args),
            DesignSubcommand::Show(args) => show(runtime, args),
        }
    }
}

fn init(_runtime: &OrbitRuntime, args: DesignInitArgs) -> Result<(), OrbitError> {
    let workspace = workspace_path(args.workspace)?;
    let owner = args.owner.unwrap_or_else(|| "human".to_string());
    let summary = init_feature(&workspace, &args.feature, &owner)?;
    println!("Created design feature: {}", summary.feature);
    print_feature_summary(&summary);
    Ok(())
}

fn list(_runtime: &OrbitRuntime, args: DesignListArgs) -> Result<(), OrbitError> {
    let workspace = workspace_path(args.workspace)?;
    let features = list_features(&workspace)?;
    let mut table =
        crate::output::table::build_table(&["FEATURE", "DOCS", "LAST_UPDATED", "STATUS"]);
    for feature in features {
        let last_updated = feature
            .docs
            .values()
            .filter_map(|doc| doc.last_updated)
            .max()
            .map(|date| date.to_string())
            .unwrap_or_else(|| "-".to_string());
        let status = if feature
            .docs
            .values()
            .any(|doc| doc.decay_status == DesignDecayStatus::Stale)
        {
            "stale"
        } else {
            "fresh"
        };
        table.add_row(vec![
            feature.feature,
            feature.docs.len().to_string(),
            last_updated,
            status.to_string(),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn show(_runtime: &OrbitRuntime, args: DesignShowArgs) -> Result<(), OrbitError> {
    let workspace = workspace_path(args.workspace)?;
    let summary = show_feature(&workspace, &args.feature)?;
    print_feature_summary(&summary);
    Ok(())
}

fn workspace_path(workspace: Option<PathBuf>) -> Result<PathBuf, OrbitError> {
    let workspace = match workspace {
        Some(workspace) => workspace,
        None => std::env::current_dir().map_err(|error| OrbitError::Io(error.to_string()))?,
    };
    Ok(workspace)
}

fn print_feature_summary(summary: &DesignFeatureSummary) {
    println!("Feature: {}", summary.feature);
    println!("Specs:   {}", summary.specs_path.display());
    println!("Refs:    {}", summary.references_path.display());
    let mut table =
        crate::output::table::build_table(&["DOC", "OWNER", "LAST_UPDATED", "STATUS", "PATH"]);
    for (name, doc) in &summary.docs {
        table.add_row(vec![
            name.clone(),
            doc.owner.clone().unwrap_or_else(|| "-".to_string()),
            doc.last_updated
                .map(|date| date.to_string())
                .unwrap_or_else(|| "-".to_string()),
            match doc.decay_status {
                DesignDecayStatus::Fresh => "fresh".to_string(),
                DesignDecayStatus::Stale => "stale".to_string(),
            },
            doc.path.display().to_string(),
        ]);
    }
    println!("{table}");
}
