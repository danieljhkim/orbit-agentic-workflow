use std::str::FromStr;

use orbit_common::types::OrbitError;
use orbit_search::{SemanticRelatedParams, SemanticSearchParams};
use orbit_store::LearningSearchParams;
use serde::Serialize;

use crate::{OrbitRuntime, SearchResult};

const DEFAULT_LIMIT: usize = 10;
const DOC_SEARCH_OVERFETCH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalSearchKind {
    Task,
    Doc,
    Learning,
    Adr,
    All,
}

impl GlobalSearchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Doc => "doc",
            Self::Learning => "learning",
            Self::Adr => "adr",
            Self::All => "all",
        }
    }

    fn includes_tasks(self) -> bool {
        matches!(self, Self::Task | Self::All)
    }

    fn includes_docs(self) -> bool {
        matches!(self, Self::Doc | Self::All)
    }

    fn includes_learnings(self) -> bool {
        matches!(self, Self::Learning | Self::All)
    }

    fn includes_adrs(self) -> bool {
        matches!(self, Self::Adr | Self::All)
    }
}

impl Default for GlobalSearchKind {
    fn default() -> Self {
        Self::All
    }
}

impl FromStr for GlobalSearchKind {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "task" => Ok(Self::Task),
            "doc" => Ok(Self::Doc),
            "learning" => Ok(Self::Learning),
            "adr" => Ok(Self::Adr),
            "all" => Ok(Self::All),
            other => Err(format!(
                "invalid search kind `{other}`; expected one of: task, doc, learning, adr, all"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GlobalSearchMode {
    Lexical,
    Semantic,
    Related,
}

#[derive(Debug, Clone)]
pub struct GlobalSearchParams {
    pub query: Option<String>,
    pub semantic: bool,
    pub related: Option<String>,
    pub kind: GlobalSearchKind,
    pub limit: usize,
    pub field: Option<String>,
    pub model: Option<String>,
}

impl GlobalSearchParams {
    pub fn normalized_limit(&self) -> usize {
        if self.limit == 0 {
            DEFAULT_LIMIT
        } else {
            self.limit
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GlobalSearchResponse {
    pub mode: GlobalSearchMode,
    pub kind: GlobalSearchKind,
    pub results: Vec<GlobalSearchHit>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GlobalSearchHit {
    pub kind: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_by: Option<Vec<String>>,
}

impl OrbitRuntime {
    pub fn global_search(
        &self,
        params: GlobalSearchParams,
    ) -> Result<GlobalSearchResponse, OrbitError> {
        let limit = params.normalized_limit();
        let mut results = Vec::new();
        let mut notes = Vec::new();

        if let Some(related_id) = params.related {
            if params
                .query
                .as_deref()
                .is_some_and(|query| !query.trim().is_empty())
            {
                return Err(OrbitError::InvalidInput(
                    "`query` and `related` are mutually exclusive".to_string(),
                ));
            }
            if !matches!(params.kind, GlobalSearchKind::Task | GlobalSearchKind::All) {
                return Err(OrbitError::InvalidInput(
                    "`related` only supports --kind task or --kind all".to_string(),
                ));
            }
            let related = self.semantic_related(SemanticRelatedParams {
                task_id: related_id,
                limit,
                model: params.model,
            })?;
            results.extend(related.results.into_iter().map(semantic_hit_to_global));
            return Ok(GlobalSearchResponse {
                mode: GlobalSearchMode::Related,
                kind: params.kind,
                results,
                notes,
            });
        }

        let query = params
            .query
            .as_deref()
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .ok_or_else(|| {
                OrbitError::InvalidInput("search query must not be empty".to_string())
            })?;
        let mode = if params.semantic {
            GlobalSearchMode::Semantic
        } else {
            GlobalSearchMode::Lexical
        };

        if params.semantic && !matches!(params.kind, GlobalSearchKind::Task) {
            notes.push(
                "semantic vector search currently runs against tasks only; docs, learnings, and ADRs use lexical matching"
                    .to_string(),
            );
        }

        if params.kind.includes_tasks() {
            if params.semantic {
                let search = self.semantic_search(SemanticSearchParams {
                    query: query.to_string(),
                    limit,
                    field: params.field,
                    kind: Some("task".to_string()),
                    model: params.model,
                })?;
                results.extend(search.results.into_iter().map(semantic_hit_to_global));
            } else {
                let mut tasks = self.search_tasks_filtered(query, &[])?;
                tasks.truncate(limit);
                results.extend(tasks.into_iter().map(|task| GlobalSearchHit {
                    kind: "task".to_string(),
                    source: "lexical".to_string(),
                    id: Some(task.id),
                    path: None,
                    title: Some(task.title),
                    summary: Some(task.description),
                    status: Some(task.status.to_string()),
                    best_field: None,
                    snippet: None,
                    score: None,
                    matched_by: None,
                }));
            }
        }

        if params.kind.includes_docs() || params.kind.includes_adrs() {
            let docs_limit = limit.saturating_mul(DOC_SEARCH_OVERFETCH).max(limit);
            let docs = self.search_docs(query, Some(docs_limit), false)?;
            for result in docs {
                match result {
                    SearchResult::Doc(result) if params.kind.includes_docs() => {
                        results.push(GlobalSearchHit {
                            kind: "doc".to_string(),
                            source: "lexical".to_string(),
                            id: None,
                            path: Some(result.record.path),
                            title: None,
                            summary: Some(result.record.summary),
                            status: Some(result.record.doc_type),
                            best_field: None,
                            snippet: None,
                            score: Some(result.score as f32),
                            matched_by: Some(result.matched_by),
                        });
                    }
                    SearchResult::Adr(result) if params.kind.includes_adrs() => {
                        results.push(GlobalSearchHit {
                            kind: "adr".to_string(),
                            source: "lexical".to_string(),
                            id: Some(result.id),
                            path: Some(result.path.to_string_lossy().into_owned()),
                            title: Some(result.title),
                            summary: None,
                            status: Some(result.status.to_string()),
                            best_field: None,
                            snippet: None,
                            score: Some(result.score as f32),
                            matched_by: Some(result.matched_by),
                        });
                    }
                    _ => {}
                }
            }
        }

        if params.kind.includes_learnings() {
            let learnings = self.search_learnings(LearningSearchParams {
                path: None,
                tag: None,
                query: Some(query.to_string()),
                limit: Some(limit),
            })?;
            results.extend(learnings.into_iter().map(|result| GlobalSearchHit {
                kind: "learning".to_string(),
                source: "lexical".to_string(),
                id: Some(result.learning.id),
                path: None,
                title: None,
                summary: Some(result.learning.summary),
                status: Some(result.learning.status.as_str().to_string()),
                best_field: None,
                snippet: None,
                score: None,
                matched_by: Some(result.matched_by),
            }));
        }

        results.truncate(limit);
        Ok(GlobalSearchResponse {
            mode,
            kind: params.kind,
            results,
            notes,
        })
    }
}

fn semantic_hit_to_global(hit: orbit_search::SemanticHit) -> GlobalSearchHit {
    GlobalSearchHit {
        kind: hit.source_kind,
        source: "semantic".to_string(),
        id: Some(hit.source_id),
        path: None,
        title: None,
        summary: None,
        status: None,
        best_field: Some(hit.best_field),
        snippet: Some(hit.snippet),
        score: Some(hit.score),
        matched_by: None,
    }
}
