use chrono::{DateTime, Utc};
use orbit_common::types::{AuditEvent, AuditEventStatus, AuditStats, OrbitError};
use orbit_store::{AuditEventFilter, AuditEventInsertParams};

use crate::OrbitRuntime;

impl OrbitRuntime {
    /// Returns persistent audit events recorded by the CLI middleware across all invocations.
    /// Backed by SQLite; survives process restarts. For in-process session events only, see
    /// [`OrbitRuntime::list_session_events`].
    pub fn list_audit_events(
        &self,
        since: Option<DateTime<Utc>>,
        tool: Option<String>,
        status: Option<AuditEventStatus>,
        role: Option<String>,
        limit: usize,
    ) -> Result<Vec<AuditEvent>, OrbitError> {
        self.stores().audit_events().list(&AuditEventFilter {
            since,
            tool_name: tool,
            status,
            role,
            limit,
        })
    }

    pub fn show_audit_event(&self, id: i64) -> Result<AuditEvent, OrbitError> {
        self.stores()
            .audit_events()
            .get(id)?
            .ok_or_else(|| OrbitError::InvalidInput(format!("audit event not found: {id}")))
    }

    pub fn prune_audit_events(&self, older_than: &DateTime<Utc>) -> Result<usize, OrbitError> {
        self.stores().audit_events().prune(older_than)
    }

    pub fn audit_event_stats(
        &self,
        since: Option<DateTime<Utc>>,
        tool: Option<String>,
    ) -> Result<AuditStats, OrbitError> {
        let (total, success_count, failure_count, denied_count, avg_duration_ms, max_duration_ms) =
            self.stores()
                .audit_events()
                .stats(since.as_ref(), tool.as_deref())?;

        let durations = self
            .stores()
            .audit_events()
            .durations(since.as_ref(), tool.as_deref())?;

        let p95_duration_ms = compute_p95(&durations);

        Ok(AuditStats {
            total,
            success_count,
            failure_count,
            denied_count,
            avg_duration_ms,
            p95_duration_ms,
            max_duration_ms,
        })
    }

    pub fn record_audit_event(&self, params: &AuditEventInsertParams) -> Result<(), OrbitError> {
        self.stores().audit_events().insert(params)
    }

    /// Hourly count buckets `(rfc3339_hour_start, count)` for audit events at or
    /// after `since`. Empty hours are NOT in the result; callers must zero-fill
    /// when rendering a sparkline that needs every bucket.
    pub fn audit_event_hourly_buckets(
        &self,
        since: &DateTime<Utc>,
    ) -> Result<Vec<(String, i64)>, OrbitError> {
        self.stores().audit_events().hourly_buckets(since)
    }

    /// `(role, count)` for `status='denied'` audit events at or after `since`,
    /// sorted desc by count. When `since` is `None`, all-time totals are returned.
    pub fn audit_denials_by_role(
        &self,
        since: Option<&DateTime<Utc>>,
    ) -> Result<Vec<(String, i64)>, OrbitError> {
        self.stores().audit_events().denials_by_role(since)
    }
}

pub(crate) fn compute_p95(sorted_durations: &[i64]) -> i64 {
    if sorted_durations.is_empty() {
        return 0;
    }
    let idx = ((sorted_durations.len() as f64) * 0.95).ceil() as usize;
    let idx = idx.min(sorted_durations.len()) - 1;
    sorted_durations[idx]
}
