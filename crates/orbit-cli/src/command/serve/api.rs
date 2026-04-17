//! Read-only JSON HTTP handlers for the dashboard.
//!
//! Each handler delegates to the same `*_to_json` helpers used by the CLI's
//! `--json` paths so the wire format stays in lockstep with the CLI.

use std::sync::Arc;

use axum::Router;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use axum::routing::get;
use orbit_core::OrbitRuntime;
use orbit_core::command::job_run::JobRunListParams;
use serde_json::{Value, json};

use crate::command::audit::audit_event_to_json;
use crate::command::job::{job_run_to_json, job_to_json_with_last_run};
use crate::command::task::output::task_to_json;

pub(super) fn router() -> Router<Arc<OrbitRuntime>> {
    Router::new()
        .route("/tasks", get(list_tasks))
        .route("/tasks/:id", get(get_task))
        .route("/jobs", get(list_jobs))
        .route("/job-runs", get(list_job_runs))
        .route("/audit", get(list_audit))
        .route("/scoreboard", get(scoreboard))
}

async fn list_tasks(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    match runtime.list_tasks() {
        Ok(tasks) => {
            let values: Vec<Value> = tasks.iter().map(task_to_json).collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

async fn get_task(State(runtime): State<Arc<OrbitRuntime>>, Path(id): Path<String>) -> Response {
    match runtime.get_task(&id) {
        Ok(task) => Json(task_to_json(&task)).into_response(),
        Err(e) => match e {
            orbit_core::OrbitError::TaskNotFound(_) => not_found(format!("task not found: {id}")),
            other => server_error(other),
        },
    }
}

async fn list_jobs(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    match runtime.list_jobs_with_last_run(true) {
        Ok(rows) => {
            let values: Vec<Value> = rows
                .iter()
                .map(|(job, last_run)| job_to_json_with_last_run(job, last_run.as_ref()))
                .collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

async fn list_job_runs(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    let params = JobRunListParams {
        job_id: None,
        state: None,
        since: None,
        limit: Some(50),
    };
    match runtime.list_job_runs(params) {
        Ok(runs) => {
            let values: Vec<Value> = runs.iter().map(job_run_to_json).collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
}

async fn list_audit(State(runtime): State<Arc<OrbitRuntime>>) -> Response {
    match runtime.list_audit_events(None, None, None, None, 100) {
        Ok(events) => {
            let values: Vec<Value> = events.iter().map(audit_event_to_json).collect();
            Json(Value::Array(values)).into_response()
        }
        Err(e) => server_error(e),
    }
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
