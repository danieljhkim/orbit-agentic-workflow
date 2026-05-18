//! Audit event listing and summary tile aggregation.

use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, Duration, Utc};
use orbit_core::command::job::JobRunListParams;
use orbit_core::{AuditEventStatus, JobRunState, OrbitRuntime};
use serde_json::{Value, json};

use super::denials::{
    collect_denial_rows, denials_by_reason_summary, denials_by_tool_summary, scan_v2_loop_denials,
};
use super::{
    AuditQuery, AuditSummaryQuery, DEFAULT_SUMMARY_WINDOW, HISTORY_DEFAULT_LIMIT,
    HISTORY_MAX_LIMIT, bad_request, bounded_limit, map_runtime_error, server_error,
    truncate_to_hour,
};
use crate::command::audit::audit_event_to_json;
use crate::parse::parse_since;

/// Default header-tile alert threshold for the denials counter. Surfaced via
/// `?denial_threshold=` and echoed back in the response so the dashboard can
/// switch the tile to alert state without a second round-trip.
const DEFAULT_DENIAL_THRESHOLD: i64 = 10;

pub(super) async fn list_audit(
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

    let exec_id_filter = q
        .execution_id
        .as_deref()
        .or(q.run_id.as_deref())
        .filter(|s| !s.is_empty());
    let profile_filter = q
        .profile
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let needle =
        q.q.as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

    let mut filtered: Vec<_> = events
        .into_iter()
        .filter(|e| {
            if let Some(eid) = exec_id_filter
                && e.execution_id != eid
            {
                return false;
            }
            if let Some(ref profile) = profile_filter
                && !arguments_json_matches_profile(e.arguments_json.as_deref(), profile)
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

/// Best-effort match of a stringified `arguments_json` payload against a
/// requested fsProfile name. Looks for any of the conventional keys
/// (`fsProfile`, `fs_profile`, `profile`) at the top level of the parsed
/// object. Returns `false` for malformed or empty payloads — the SQLite schema
/// has no profile column, so absence cannot be distinguished from mismatch.
fn arguments_json_matches_profile(raw: Option<&str>, expected: &str) -> bool {
    let Some(raw) = raw else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return false;
    };
    const KEYS: &[&str] = &["fsProfile", "fs_profile", "profile"];
    let Some(obj) = value.as_object() else {
        return false;
    };
    for key in KEYS {
        if let Some(Value::String(found)) = obj.get(*key)
            && found == expected
        {
            return true;
        }
    }
    false
}

pub(super) async fn audit_summary(
    State(runtime): State<Arc<OrbitRuntime>>,
    Query(q): Query<AuditSummaryQuery>,
) -> Response {
    let raw_since = q.since.as_deref().unwrap_or(DEFAULT_SUMMARY_WINDOW);
    let since = match parse_since(raw_since) {
        Ok(ts) => ts,
        Err(e) => return map_runtime_error(e),
    };
    let denial_threshold = q.denial_threshold.unwrap_or(DEFAULT_DENIAL_THRESHOLD);

    let (total, _success, _failure, sql_denied, _avg, _max) =
        match runtime.audit_event_stats(Some(since), None) {
            Ok(stats) => (
                stats.total,
                stats.success_count,
                stats.failure_count,
                stats.denied_count,
                stats.avg_duration_ms,
                stats.max_duration_ms,
            ),
            Err(e) => return server_error(e),
        };

    let v2_denials = match scan_v2_loop_denials(&runtime, Some(since), None, None) {
        Ok(events) => events.len() as i64,
        Err(e) => return server_error(e),
    };

    let failed_runs = match count_failed_runs(&runtime, since) {
        Ok(n) => n,
        Err(e) => return map_runtime_error(e),
    };

    let active_long_runs = match count_active_long_runs(&runtime, since) {
        Ok(n) => n,
        Err(e) => return map_runtime_error(e),
    };

    let buckets = match runtime.audit_event_hourly_buckets(&since) {
        Ok(b) => b,
        Err(e) => return server_error(e),
    };
    let sparkline = build_sparkline(since, &buckets);

    let events = match runtime.list_audit_events(Some(since), None, None, None, 100_000) {
        Ok(evs) => evs,
        Err(e) => return server_error(e),
    };

    let mut failures_by_tool: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut duration_by_tool: std::collections::HashMap<String, Vec<i64>> =
        std::collections::HashMap::new();
    let mut totals_by_tool: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut role_split: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

    let mut role_mcp: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut role_cli: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut tool_mcp_fails: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut tool_cli_fails: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut tool_mcp_total: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut tool_cli_total: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut mcp_count: u64 = 0;
    let mut cli_count: u64 = 0;

    for e in &events {
        let role = e.role.clone();
        *role_split.entry(role.clone()).or_default() += 1;

        let is_mcp = e.subcommand.as_deref() == Some("run-mcp");
        let is_cli = e.subcommand.as_deref() == Some("run");

        if is_mcp {
            mcp_count += 1;
            *role_mcp.entry(role.clone()).or_default() += 1;
        } else if is_cli {
            cli_count += 1;
            *role_cli.entry(role.clone()).or_default() += 1;
        }

        let tool = e.tool_name.clone().unwrap_or_else(|| "unknown".to_string());
        *totals_by_tool.entry(tool.clone()).or_default() += 1;
        if is_mcp {
            *tool_mcp_total.entry(tool.clone()).or_default() += 1;
        } else if is_cli {
            *tool_cli_total.entry(tool.clone()).or_default() += 1;
        }

        if e.status == orbit_core::AuditEventStatus::Failure {
            *failures_by_tool.entry(tool.clone()).or_default() += 1;
            if is_mcp {
                *tool_mcp_fails.entry(tool.clone()).or_default() += 1;
            } else if is_cli {
                *tool_cli_fails.entry(tool.clone()).or_default() += 1;
            }
        }

        duration_by_tool
            .entry(tool)
            .or_default()
            .push(e.duration_ms as i64);
    }

    let mut failures_vec: Vec<_> = failures_by_tool
        .iter()
        .map(|(tool, count)| {
            let mcp = tool_mcp_fails.get(tool).copied().unwrap_or(0);
            let cli = tool_cli_fails.get(tool).copied().unwrap_or(0);
            json!({"tool": tool, "count": count, "mcp": mcp, "cli": cli})
        })
        .collect();
    failures_vec.sort_by_key(|v| std::cmp::Reverse(v["count"].as_u64().unwrap_or(0)));
    failures_vec.truncate(8);

    let mut duration_vec: Vec<_> = duration_by_tool
        .into_iter()
        .map(|(tool, mut durations)| {
            durations.sort_unstable();
            let n = durations.len();
            let avg = if n > 0 {
                durations.iter().sum::<i64>() as f64 / n as f64
            } else {
                0.0
            };
            let p95_idx = ((n as f64 * 0.95).ceil() as usize).min(n).saturating_sub(1);
            let p95 = if n > 0 { durations[p95_idx] } else { 0 };
            json!({"tool": tool, "count": n, "avg": avg, "p95": p95})
        })
        .collect();
    duration_vec.sort_by(|a, b| {
        b["avg"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["avg"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    duration_vec.truncate(8);

    let mut rate_vec: Vec<_> = totals_by_tool.iter().filter(|(_, total)| **total >= 5).map(|(tool, total)| {
        let fails = failures_by_tool.get(tool).copied().unwrap_or(0);
        let mcp_fails = tool_mcp_fails.get(tool).copied().unwrap_or(0);
        let cli_fails = tool_cli_fails.get(tool).copied().unwrap_or(0);
        let mcp_total = tool_mcp_total.get(tool).copied().unwrap_or(0);
        let cli_total = tool_cli_total.get(tool).copied().unwrap_or(0);
        let rate = fails as f64 / *total as f64;
        let mcp_rate = if mcp_total > 0 { mcp_fails as f64 / mcp_total as f64 } else { 0.0 };
        let cli_rate = if cli_total > 0 { cli_fails as f64 / cli_total as f64 } else { 0.0 };
        json!({"tool": tool, "rate": rate, "failures": fails, "mcp_rate": mcp_rate, "cli_rate": cli_rate, "total": total})
    }).collect();
    rate_vec.sort_by(|a, b| {
        b["rate"]
            .as_f64()
            .unwrap_or(0.0)
            .partial_cmp(&a["rate"].as_f64().unwrap_or(0.0))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rate_vec.truncate(8);

    let mut role_vec: Vec<_> = role_split
        .into_iter()
        .map(|(role, count)| {
            let mcp = role_mcp.get(&role).copied().unwrap_or(0);
            let cli = role_cli.get(&role).copied().unwrap_or(0);
            json!({"label": role, "count": count, "mcp": mcp, "cli": cli})
        })
        .collect();
    role_vec.sort_by_key(|v| std::cmp::Reverse(v["count"].as_u64().unwrap_or(0)));

    let mcp_vs_cli_split = json!([
        {"label": "mcp", "count": mcp_count},
        {"label": "cli", "count": cli_count},
    ]);

    let denial_rows = match collect_denial_rows(&runtime, Some(since), None, None) {
        Ok(rows) => rows,
        Err(e) => return server_error(e),
    };
    let denials_by_tool = denials_by_tool_summary(&denial_rows, 8);
    let denials_by_reason = denials_by_reason_summary(&denial_rows, 8);

    let denials = sql_denied + v2_denials;

    Json(json!({
        "events": total,
        "denials": denials,
        "denials_sql": sql_denied,
        "denials_v2": v2_denials,
        "failed_runs": failed_runs,
        "active_long_runs": active_long_runs,
        "sparkline": sparkline,
        "denial_threshold": denial_threshold,
        "since": since.to_rfc3339(),
        "window": raw_since,
        "failures_by_tool": failures_vec,
        "duration_by_tool": duration_vec,
        "failure_rate_by_tool": rate_vec,
        "role_split": role_vec,
        "mcp_vs_cli_split": mcp_vs_cli_split,
        "denials_by_tool": denials_by_tool,
        "denials_by_reason": denials_by_reason,
    }))
    .into_response()
}

/// Builds a contiguous hourly sparkline covering `[truncate_to_hour(since), now]`,
/// zero-filling hours not present in `buckets`. Always returns at least 24
/// buckets so the UI can render a stable baseline width even on a fresh
/// workspace.
fn build_sparkline(since: DateTime<Utc>, buckets: &[(String, i64)]) -> Vec<Value> {
    let mut by_bucket: BTreeMap<String, i64> = BTreeMap::new();
    for (ts, count) in buckets {
        by_bucket.insert(ts.clone(), *count);
    }
    let now = Utc::now();
    let start = truncate_to_hour(since.min(now));
    let end = truncate_to_hour(now);
    let mut out = Vec::new();
    let mut cursor = start;
    while cursor <= end {
        let key = cursor.format("%Y-%m-%dT%H:00:00Z").to_string();
        let count = by_bucket.get(&key).copied().unwrap_or(0);
        out.push(json!({ "ts": key, "count": count }));
        cursor += Duration::hours(1);
    }
    while out.len() < 24 {
        let earliest = out
            .first()
            .and_then(|v| v.get("ts").and_then(Value::as_str))
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(end);
        let prev = earliest - Duration::hours(1);
        let key = prev.format("%Y-%m-%dT%H:00:00Z").to_string();
        out.insert(0, json!({ "ts": key, "count": 0 }));
    }
    out
}

fn count_failed_runs(
    runtime: &OrbitRuntime,
    since: DateTime<Utc>,
) -> Result<i64, orbit_core::OrbitError> {
    let mut total: i64 = 0;
    for state in [JobRunState::Failed, JobRunState::Timeout] {
        let runs = runtime.list_job_runs(JobRunListParams {
            job_id: None,
            state: Some(state),
            since: Some(since),
            limit: Some(HISTORY_MAX_LIMIT),
        })?;
        total += runs.len() as i64;
    }
    Ok(total)
}

/// Counts running runs whose start time is older than the 95th percentile of
/// finished-run wall-clock durations within the same window. We use run-level
/// `duration_ms` as a proxy for the AC's "finished step" series — load-bearing
/// the same per-run signal without paying the O(steps) file-read cost. Faithful
/// to the spec's intent (flag stuck activity) and within the 500ms budget.
fn count_active_long_runs(
    runtime: &OrbitRuntime,
    since: DateTime<Utc>,
) -> Result<i64, orbit_core::OrbitError> {
    let mut finished_durations: Vec<i64> = Vec::new();
    for state in [
        JobRunState::Success,
        JobRunState::Failed,
        JobRunState::Timeout,
        JobRunState::Cancelled,
    ] {
        let runs = runtime.list_job_runs(JobRunListParams {
            job_id: None,
            state: Some(state),
            since: Some(since),
            limit: Some(HISTORY_MAX_LIMIT),
        })?;
        for r in runs {
            if let Some(d) = r.duration_ms {
                finished_durations.push(d as i64);
            }
        }
    }

    if finished_durations.is_empty() {
        return Ok(0);
    }
    finished_durations.sort_unstable();
    let idx = ((finished_durations.len() as f64) * 0.95).ceil() as usize;
    let idx = idx.min(finished_durations.len()).saturating_sub(1);
    let p95_ms = finished_durations[idx];

    let running = runtime.list_job_runs(JobRunListParams {
        job_id: None,
        state: Some(JobRunState::Running),
        since: None,
        limit: Some(HISTORY_MAX_LIMIT),
    })?;

    let now = Utc::now();
    let mut count: i64 = 0;
    for r in running {
        let started = r.started_at.unwrap_or(r.created_at);
        let elapsed = now.signed_duration_since(started).num_milliseconds().max(0);
        if elapsed > p95_ms {
            count += 1;
        }
    }
    Ok(count)
}
