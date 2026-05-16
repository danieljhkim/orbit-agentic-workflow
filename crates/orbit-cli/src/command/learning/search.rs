use clap::Args;
use orbit_core::{LearningSearchParams, OrbitError, OrbitRuntime};
use serde_json::Value;

use crate::command::Execute;

use super::output::learning_search_result_to_json;

#[derive(Args)]
pub struct LearningSearchArgs {
    /// Test this path against every learning's scope paths
    #[arg(long)]
    pub path: Option<String>,
    /// Test this tag against every learning's scope tags
    #[arg(long)]
    pub tag: Option<String>,
    /// Substring match against learning summaries
    #[arg(long)]
    pub query: Option<String>,
    /// Cap on returned rows
    #[arg(long)]
    pub limit: Option<usize>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for LearningSearchArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let results = runtime.search_learnings(LearningSearchParams {
            path: self.path,
            tag: self.tag,
            query: self.query,
            limit: self.limit,
        })?;

        if self.json {
            let array = Value::Array(results.iter().map(learning_search_result_to_json).collect());
            crate::output::json::print_pretty(&array)
        } else {
            for result in &results {
                println!(
                    "{}\t{}\t[{}]",
                    result.learning.id,
                    result.learning.summary,
                    result.matched_by.join(", "),
                );
            }
            Ok(())
        }
    }
}
