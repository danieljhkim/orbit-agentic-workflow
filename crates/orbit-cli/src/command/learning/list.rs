use std::str::FromStr;

use clap::Args;
use orbit_core::{LearningStatus, OrbitError, OrbitRuntime};
use serde_json::Value;

use crate::command::Execute;

use super::output::learning_to_json;

#[derive(Args)]
pub struct LearningListArgs {
    /// Filter by status (active | superseded). Defaults to all.
    #[arg(long)]
    pub status: Option<String>,
    /// Filter to learnings whose scope tags contain this tag
    #[arg(long)]
    pub tag: Option<String>,
    /// Filter to learnings whose scope paths include this glob (exact match)
    #[arg(long)]
    pub path: Option<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for LearningListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let status = self
            .status
            .as_deref()
            .map(|raw| LearningStatus::from_str(raw).map_err(OrbitError::InvalidInput))
            .transpose()?;
        let tag = self.tag.as_deref().map(|t| t.trim().to_lowercase());

        let learnings = runtime.list_learnings(status)?;
        let filtered: Vec<_> = learnings
            .into_iter()
            .filter(|l| {
                if let Some(ref tag) = tag
                    && !l.scope.tags.iter().any(|t| t == tag)
                {
                    return false;
                }
                if let Some(ref path) = self.path
                    && !l.scope.paths.iter().any(|p| p == path)
                {
                    return false;
                }
                true
            })
            .collect();

        if self.json {
            let array = Value::Array(filtered.iter().map(learning_to_json).collect());
            crate::output::json::print_pretty(&array)
        } else {
            for learning in &filtered {
                println!(
                    "{}\t{}\t{}",
                    learning.id,
                    learning.status.as_str(),
                    learning.summary
                );
            }
            Ok(())
        }
    }
}
