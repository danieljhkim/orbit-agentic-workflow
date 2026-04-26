//! Read-only JSON HTTP handlers for the dashboard.
//!
//! Each handler delegates to the same `*_to_json` helpers used by the CLI's
//! `--json` paths so the wire format stays in lockstep with the CLI.

use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use chrono::Utc;
use orbit_core::command::job_run::JobRunListParams;
use orbit_core::{AuditEventStatus, OrbitRuntime, Task, TaskStatus};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::command::audit::audit_event_to_json;
use crate::command::job::job_catalog_to_json_with_last_run;
use crate::command::run::job_run_to_json;
use crate::command::task::output::task_to_json;
use crate::parse::parse_since;

const DASHBOARD_TASK_STATUSES: &[TaskStatus] = &[
    TaskStatus::InProgress,
    TaskStatus::Review,
    TaskStatus::Blocked,
    TaskStatus::Proposed,
    TaskStatus::Backlog,
    TaskStatus::Someday,
    TaskStatus::Rejected,
];
const HISTORY_DEFAULT_LIMIT: usize = 50;
const JOB_RUN_DEFAULT_LIMIT: usize = 25;
const HISTORY_MAX_LIMIT: usize = 200;
const RUN_EVENTS_DEFAULT_LIMIT: usize = 100;

#[derive(Deserialize, Default)]
pub(super) struct LimitQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize)]
pub(super) struct DiagnosticsQuery {
    #[serde(default)]
    month: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Deserialize, Default)]
pub(super) struct AuditQuery {
    #[serde(default)]
    since: Option<String>,
    #[serde(default)]
    tool: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

#[derive(Deserialize, Default)]
pub(super) struct RunEventsQuery {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
}

fn current_year_month_utc() -> String {
    Utc::now().format("%Y-%m").to_string()
}

/// Validates a `YYYY-MM` string with month range 01..=12.
fn validate_year_month(raw: &str) -> Result<(), orbit_core::OrbitError> {
    let bytes = raw.as_bytes();
    let format_ok = bytes.len() == 7
        && bytes[4] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..].iter().all(u8::is_ascii_digit);
    if !format_ok {
        return Err(orbit_core::OrbitError::InvalidInput(format!(
            "month must be in YYYY-MM format, got '{raw}'"
        )));
    }
    let month: u32 = raw[5..].parse().unwrap_or(0);
    if !(1..=12).contains(&month) {
        return Err(orbit_core::OrbitError::InvalidInput(format!(
            "month component must be 01-12, got '{raw}'"
        )));
    }
    Ok(())
}

async fn require_localhost_origin(request: Request<Body>, next: Next) -> Response {
    if !request.method().is_safe()
        && let Some(origin) = request.headers().get(header::ORIGIN)
    {
        let origin = origin.to_str().unwrap_or("");
        if !origin.starts_with("http://localhost") && !origin.starts_with("http://127.0.0.1") {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({"error": "cross-origin requests not allowed"})),
            )
                .into_response();
        }
    }
    next.run(request).await
}

pub(super) fn router() -> Router<Arc<OrbitRuntime>> {
    Router::new()
        .route("/tasks", get(list_tasks))
        .route("/tasks/:id", get(get_task))
        .route("/tasks/:id/approve", post(approve_task_action))
        .route("/tasks/:id/reject", post(reject_task_action))
        .route("/tasks/:id/archive", post(archive_task_action))
        .route("/jobs", get(list_jobs))
        .route("/job-runs", get(list_job_runs))
        .route("/runs/:id", get(get_run))
        .route("/runs/:id/events", get(list_run_events))
        .route("/audit", get(list_audit))
        .route("/scoreboard", get(scoreboard))
        .route("/diagnostics/metrics", get(list_diagnostics_metrics))
        .route("/diagnostics/friction", get(list_diagnostics_friction))
        .layer(middleware::from_fn(require_localhost_origin))
}

#[derive(Deserialize, Default)]
pub(super) struct ApproveBody {
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    comment: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct RejectBody {
    note: String,
    #[serde(default)]
    comment: Option<String>,
}

async fn list_tasks(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    match list_dashboard_tasks(&runtime) {
        Ok(tasks) => {
            let status_by_id = orbit_core::build_task_status_index(&tasks);
            let values: Vec<Value> = tasks
                .iter()
                .map(|task| task_to_json(task, &status_by_id))
                .collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

fn list_dashboard_tasks(runtime: &OrbitRuntime) -> Result<Vec<Task>, orbit_core::OrbitError> {
    let mut tasks = Vec::new();
    for status in DASHBOARD_TASK_STATUSES {
        tasks.extend(runtime.list_tasks_filtered(Some(*status), None, None, None)?);
    }
    Ok(tasks)
}

fn dashboard_status_index(
    runtime: &OrbitRuntime,
) -> Result<std::collections::BTreeMap<String, TaskStatus>, orbit_core::OrbitError> {
    Ok(orbit_core::build_task_status_index(&list_dashboard_tasks(
        runtime,
    )?))
}

fn bounded_limit(requested: Option<usize>, default: usize) -> usize {
    requested.unwrap_or(default).min(HISTORY_MAX_LIMIT)
}

async fn get_task(State(runtime): State<Arc<OrbitRuntime>>, Path(id): Path<String>) -> Response {
    match runtime.get_task(&id) {
        Ok(task) => match dashboard_status_index(&runtime) {
            Ok(status_by_id) => Json(task_to_json(&task, &status_by_id)).into_response(),
            Err(e) => server_error(e),
        },
        Err(e) => map_runtime_error(e),
    }
}

async fn approve_task_action(
    State(runtime): State<Arc<OrbitRuntime>>,
    Path(id): Path<String>,
    body: Option<Json<ApproveBody>>,
) -> Response {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    match runtime.approve_task(&id, body.note, body.comment) {
        Ok(task) => match dashboard_status_index(&runtime) {
            Ok(status_by_id) => Json(task_to_json(&task, &status_by_id)).into_response(),
            Err(e) => server_error(e),
        },
        Err(e) => map_runtime_error(e),
    }
}

async fn reject_task_action(
    State(runtime): State<Arc<OrbitRuntime>>,
    Path(id): Path<String>,
    Json(body): Json<RejectBody>,
) -> Response {
    match runtime.reject_task(&id, body.note, body.comment) {
        Ok(task) => match dashboard_status_index(&runtime) {
            Ok(status_by_id) => Json(task_to_json(&task, &status_by_id)).into_response(),
            Err(e) => server_error(e),
        },
        Err(e) => map_runtime_error(e),
    }
}

async fn archive_task_action(
    State(runtime): State<Arc<OrbitRuntime>>,
    Path(id): Path<String>,
) -> Response {
    match runtime.archive_task(&id) {
        Ok(()) => Json(json!({ "ok": true, "id": id })).into_response(),
        Err(e) => map_runtime_error(e),
    }
}

async fn list_jobs(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    use orbit_core::command::job::JobCatalogFilter;
    match runtime.list_job_catalog_with_last_run(true, JobCatalogFilter::All) {
        Ok(rows) => {
            let values: Vec<Value> = rows
                .iter()
                .map(|(entry, last_run)| {
                    job_catalog_to_json_with_last_run(entry, last_run.as_ref())
                })
                .collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

async fn list_job_runs(
    State(runtime): State<Arc<OrbitRuntime>>,
    Query(q): Query<LimitQuery>,
) -> Response {
    let limit = bounded_limit(q.limit, JOB_RUN_DEFAULT_LIMIT);
    let params = JobRunListParams {
        job_id: None,
        state: None,
        since: None,
        limit: Some(limit),
    };
    match runtime.list_job_runs(params) {
        Ok(runs) => {
            let values: Vec<Value> = runs.iter().map(job_run_to_json).collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

async fn list_audit(
    State(runtime): State<Arc<OrbitRuntime>>,
    Query(q): Query<AuditQuery>,
) -> Response {
    let since = match q.since.as_deref() {
        Some(raw) => match parse_since(raw) {
            Ok(ts) => Some(ts),
            Err(e) => return map_runtime_error(e),
        },
        None => None,
    };

    let status = match q.status.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => match AuditEventStatus::from_str(raw) {
            Ok(s) => Some(s),
            Err(msg) => return bad_request(msg),
        },
        None => None,
    };

    let limit = bounded_limit(q.limit, HISTORY_DEFAULT_LIMIT);
    let offset = q.offset.unwrap_or(0);

    // Post-query filters (run_id, q) and offset are applied after the SQLite
    // call. Request a generous prefetch so common filtered pages are full.
    let prefetch = HISTORY_MAX_LIMIT;
    let tool = q.tool.filter(|s| !s.is_empty());
    let role = q.role.filter(|s| !s.is_empty());

    let events = match runtime.list_audit_events(since, tool, status, role, prefetch) {
        Ok(events) => events,
        Err(e) => return server_error(e),
    };

    let run_id_filter = q.run_id.as_deref().filter(|s| !s.is_empty());
    let needle =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

    let mut filtered: Vec<_> = events
        .into_iter()
        .filter(|e| {
            if let Some(rid) = run_id_filter
                && e.execution_id != rid
            {
                return false;
            }
            if let Some(ref needle) = needle {
                let haystacks = [
                    e.command.as_str(),
                    e.subcommand.as_deref().unwrap_or(""),
                    e.tool_name.as_deref().unwrap_or(""),
                    e.target_id.as_deref().unwrap_or(""),
                    e.target_type.as_deref().unwrap_or(""),
                    e.role.as_str(),
                    e.error_message.as_deref().unwrap_or(""),
                ];
                if !haystacks.iter().any(|h| h.to_lowercase().contains(needle)) {
                    return false;
                }
            }
            true
        })
        .collect();

    if offset >= filtered.len() {
        return Json(Value::Array(Vec::new())).into_response();
    }
    let end = offset.saturating_add(limit).min(filtered.len());
    let page: Vec<Value> = filtered
        .drain(offset..end)
        .map(|e| audit_event_to_json(&e))
        .collect();
    Json(Value::Array(page)).into_response()
}

async fn get_run(State(runtime): State<Arc<OrbitRuntime>>, Path(id): Path<String>) -> Response {
    match runtime.show_job_run(&id) {
        Ok(run) => {
            let mut full = job_run_to_json(&run);
            // Reshape into `{run, steps}` per the dashboard contract: peel the
            // `steps` array off the flat `job_run_to_json` output.
            let steps = full
                .as_object_mut()
                .and_then(|m| m.remove("steps"))
                .unwrap_or(Value::Array(Vec::new()));
            Json(json!({ "run": full, "steps": steps })).into_response()
        }
        Err(e) => map_runtime_error(e),
    }
}

async fn list_run_events(
    State(runtime): State<Arc<OrbitRuntime>>,
    Path(id): Path<String>,
    Query(q): Query<RunEventsQuery>,
) -> Response {
    let limit = bounded_limit(q.limit, RUN_EVENTS_DEFAULT_LIMIT);
    let offset = q.offset.unwrap_or(0);
    let kind = q
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let path = v2_loop_path(&runtime, &id);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Json(Value::Array(Vec::new())).into_response();
        }
        Err(e) => {
            return server_error(orbit_core::OrbitError::Io(format!(
                "read {}: {e}",
                path.display()
            )));
        }
    };

    let mut events: Vec<Value> = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(ref needle) = kind {
            let body_kind = value.get("body_kind").and_then(Value::as_str).unwrap_or("");
            if body_kind != needle {
                continue;
            }
        }
        events.push(value);
    }

    if offset >= events.len() {
        return Json(Value::Array(Vec::new())).into_response();
    }
    let end = offset.saturating_add(limit).min(events.len());
    let page: Vec<Value> = events.drain(offset..end).collect();
    Json(Value::Array(page)).into_response()
}

fn v2_loop_path(runtime: &OrbitRuntime, run_id: &str) -> PathBuf {
    runtime
        .data_root()
        .join("state")
        .join("audit")
        .join("v2_loop")
        .join(format!("{run_id}.jsonl"))
}

async fn scoreboard(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    match runtime.generate_scoreboard_summary() {
        Ok(summary) => match serde_json::to_value(&summary) {
            Ok(value) => Json(value).into_response(),
            Err(e) => server_error(orbit_core::OrbitError::Store(e.to_string())),
        },
        Err(e) => server_error(e),
    }
}

async fn list_diagnostics_metrics(
    State(runtime): State<Arc<OrbitRuntime>>,
    Query(q): Query<DiagnosticsQuery>,
) -> Response {
    let month = q.month.unwrap_or_else(current_year_month_utc);
    if let Err(e) = validate_year_month(&month) {
        return map_runtime_error(e);
    }
    let limit = bounded_limit(q.limit, HISTORY_DEFAULT_LIMIT);
    match runtime.read_metrics_entries_limited(&month, limit) {
        Ok(mut entries) => {
            entries.sort_by(|a, b| b.ts.cmp(&a.ts));
            entries.truncate(limit);
            match serde_json::to_value(&entries) {
                Ok(value) => Json(value).into_response(),
                Err(e) => server_error(orbit_core::OrbitError::Store(e.to_string())),
            }
        }
        Err(e) => map_runtime_error(e),
    }
}

async fn list_diagnostics_friction(
    State(runtime): State<Arc<OrbitRuntime>>,
    Query(q): Query<DiagnosticsQuery>,
) -> Response {
    let month = q.month.unwrap_or_else(current_year_month_utc);
    if let Err(e) = validate_year_month(&month) {
        return map_runtime_error(e);
    }
    let limit = bounded_limit(q.limit, HISTORY_DEFAULT_LIMIT);
    match runtime.read_friction_entries_limited(&month, limit) {
        Ok(mut entries) => {
            entries.sort_by(|a, b| b.ts.cmp(&a.ts));
            entries.truncate(limit);
            match serde_json::to_value(&entries) {
                Ok(value) => Json(value).into_response(),
                Err(e) => server_error(orbit_core::OrbitError::Store(e.to_string())),
            }
        }
        Err(e) => map_runtime_error(e),
    }
}

fn map_runtime_error(e: orbit_core::OrbitError) -> Response {
    match e {
        orbit_core::OrbitError::InvalidInput(msg) => bad_request(msg),
        orbit_core::OrbitError::TaskNotFound(msg) => not_found(format!("task not found: {msg}")),
        orbit_core::OrbitError::JobRunNotFound(msg) => not_found(format!("run not found: {msg}")),
        other => server_error(other),
    }
}

fn bad_request(message: String) -> Response {
    (StatusCode::BAD_REQUEST, Json(json!({ "error": message }))).into_response()
}

fn not_found(message: String) -> Response {
    (StatusCode::NOT_FOUND, Json(json!({ "error": message }))).into_response()
}

fn server_error(e: orbit_core::OrbitError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": e.to_string() })),
    )
        .into_response()
}
