mod error;
pub mod extract;
pub mod graph;
mod io;
pub mod lock;
pub mod pipeline;
mod selector;
pub mod service;
mod store;
pub mod working_graph;

#[doc(hidden)]
pub mod task_commits {
    //! Sidecar-index re-export so integration tests and downstream crates can
    //! read the task-commits index without reaching into `store` directly.
    pub use crate::store::task_commits::*;
}

pub use error::KnowledgeError;
pub use selector::{Selector, SelectorParseError};
pub use service::history::{
    CommitSummary as HistoryCommitSummary, DEFAULT_STALENESS_THRESHOLD, HistoryQueryOptions,
    HistorySource, StalenessInfo, TaskHistoryEntry, TaskHistoryResult, query_task_history,
};
pub use service::{TaskGraphScope, TaskGraphService, default_knowledge_dir};
pub use store::{
    KnowledgeEntryKind, KnowledgePack, KnowledgePackEntry, KnowledgeStore, LeafData, NodeTaskInfo,
    SymbolSummary, load_task_working_graph, overlay_pack_with_working_graph,
    pack_from_working_graph, save_task_working_graph, task_working_graph_state_path,
};
pub use working_graph::{
    LeafEdit, LeafVersionChain, MoveResult, WorkingGraph, WorkingLeaf, WriteError, WriteResult,
};
