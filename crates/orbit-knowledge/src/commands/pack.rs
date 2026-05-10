use serde_json::Value;

use crate::commands::{GraphCommandContext, knowledge_error_from_orbit};
use crate::graph::GraphReadOptions;
use crate::{KnowledgeError, Selector};

#[derive(Debug, Clone)]
pub struct PackInput {
    pub context: GraphCommandContext,
    pub selectors: Vec<String>,
    pub hydrate_leaf_source: bool,
    pub refresh: bool,
    pub selector_timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PackResult {
    pub pack: Value,
}

pub fn run(input: PackInput) -> Result<PackResult, KnowledgeError> {
    let selectors = Selector::parse_many(&input.selectors)
        .map_err(|error| KnowledgeError::invalid_data(error.to_string()))?;
    let service = input.context.task_service();
    let skip_auto_refresh = !input.refresh || input.context.explicit_knowledge_dir;
    let pack = service
        .pack_json(
            &selectors,
            input.context.workspace_root.as_deref(),
            skip_auto_refresh,
            input.context.explicit_ref.as_deref(),
            GraphReadOptions {
                hydrate_leaf_source: input.hydrate_leaf_source,
                ..Default::default()
            },
            Some(input.selector_timeout_ms),
        )
        .map_err(knowledge_error_from_orbit)?;

    Ok(PackResult { pack })
}
