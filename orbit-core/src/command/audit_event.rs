use chrono::{DateTime, Utc};
use orbit_store::{AuditEventFilter, AuditEventInsertParams};
use orbit_types::{AuditEvent, AuditEventStatus, AuditStats, OrbitError};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn list_audit_events(
        &self,
        since: Option<DateTime<Utc>>,
        tool: Option<String>,
        status: Option<AuditEventStatus>,
        role: Option<String>,
        limit: usize,
    ) -> Result<Vec<AuditEvent>, OrbitError> {
        self.context
            .audit_event_store
            .list_audit_events(&AuditEventFilter {
                since,
                tool_name: tool,
                status,
                role,
                limit,
            })
    }

    pub fn show_audit_event(&self, id: i64) -> Result<AuditEvent, OrbitError> {
        self.context
            .audit_event_store
            .get_audit_event(id)?
            .ok_or_else(|| OrbitError::InvalidInput(format!("audit event not found: {id}")))
    }

    pub fn prune_audit_events(&self, older_than: &DateTime<Utc>) -> Result<usize, OrbitError> {
        self.context
            .audit_event_store
            .prune_audit_events(older_than)
    }

    pub fn audit_event_stats(
        &self,
        since: Option<DateTime<Utc>>,
        tool: Option<String>,
    ) -> Result<AuditStats, OrbitError> {
        let (total, success_count, failure_count, denied_count, avg_duration_ms, max_duration_ms) =
            self.context
                .audit_event_store
                .get_audit_event_stats(since.as_ref(), tool.as_deref())?;

        let durations = self
            .context
            .audit_event_store
            .get_audit_event_durations(since.as_ref(), tool.as_deref())?;

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
        self.context
            .audit_event_store
            .insert_audit_event_record(params)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn p95_known_values() {
        let durations = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        assert_eq!(compute_p95(&durations), 100);
    }

    #[test]
    fn p95_single_value() {
        assert_eq!(compute_p95(&[42]), 42);
    }

    #[test]
    fn p95_empty() {
        assert_eq!(compute_p95(&[]), 0);
    }

    #[test]
    fn p95_twenty_values() {
        let durations: Vec<i64> = (1..=20).collect();
        // 95% of 20 = 19.0 -> ceil = 19 -> idx 18 -> value 19
        assert_eq!(compute_p95(&durations), 19);
    }

    #[test]
    fn record_and_list_via_runtime() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");

        runtime
            .record_audit_event(&AuditEventInsertParams {
                execution_id: "exec-rt-1".to_string(),
                command: "tool".to_string(),
                subcommand: Some("run".to_string()),
                tool_name: Some("fs.read".to_string()),
                target_type: None,
                target_id: None,
                role: "admin".to_string(),
                status: AuditEventStatus::Success,
                exit_code: 0,
                duration_ms: 50,
                working_directory: "/tmp".to_string(),
                arguments_json: None,
                stdout_truncated: None,
                stderr_truncated: None,
                error_message: None,
                host: None,
                pid: 1,
                session_id: None,
            })
            .expect("record");

        let events = runtime
            .list_audit_events(None, None, None, None, 10)
            .expect("list");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].execution_id, "exec-rt-1");
    }

    #[test]
    fn show_not_found_returns_error() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let result = runtime.show_audit_event(999);
        assert!(result.is_err());
    }

    #[test]
    fn stats_empty_returns_zeros() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");
        let stats = runtime.audit_event_stats(None, None).expect("stats");
        assert_eq!(stats.total, 0);
        assert_eq!(stats.success_count, 0);
        assert_eq!(stats.failure_count, 0);
        assert_eq!(stats.denied_count, 0);
        assert_eq!(stats.p95_duration_ms, 0);
    }

    #[test]
    fn stats_via_runtime() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");

        for (i, (status, dur)) in [
            (AuditEventStatus::Success, 100),
            (AuditEventStatus::Failure, 200),
            (AuditEventStatus::Success, 300),
        ]
        .iter()
        .enumerate()
        {
            runtime
                .record_audit_event(&AuditEventInsertParams {
                    execution_id: format!("exec-stat-{i}"),
                    command: "tool".to_string(),
                    subcommand: None,
                    tool_name: None,
                    target_type: None,
                    target_id: None,
                    role: "admin".to_string(),
                    status: *status,
                    exit_code: 0,
                    duration_ms: *dur,
                    working_directory: "/tmp".to_string(),
                    arguments_json: None,
                    stdout_truncated: None,
                    stderr_truncated: None,
                    error_message: None,
                    host: None,
                    pid: 1,
                    session_id: None,
                })
                .expect("record");
        }

        let stats = runtime.audit_event_stats(None, None).expect("stats");
        assert_eq!(stats.total, 3);
        assert_eq!(stats.success_count, 2);
        assert_eq!(stats.failure_count, 1);
        assert_eq!(stats.denied_count, 0);
        assert_eq!(stats.max_duration_ms, 300);
    }

    #[test]
    fn prune_via_runtime() {
        let runtime = OrbitRuntime::in_memory().expect("runtime");

        runtime
            .record_audit_event(&AuditEventInsertParams {
                execution_id: "exec-prune-1".to_string(),
                command: "tool".to_string(),
                subcommand: None,
                tool_name: None,
                target_type: None,
                target_id: None,
                role: "admin".to_string(),
                status: AuditEventStatus::Success,
                exit_code: 0,
                duration_ms: 10,
                working_directory: "/tmp".to_string(),
                arguments_json: None,
                stdout_truncated: None,
                stderr_truncated: None,
                error_message: None,
                host: None,
                pid: 1,
                session_id: None,
            })
            .expect("record");

        // Prune everything older than far future
        let future = Utc::now() + chrono::Duration::days(1);
        let pruned = runtime.prune_audit_events(&future).expect("prune");
        assert_eq!(pruned, 1);

        let events = runtime
            .list_audit_events(None, None, None, None, 10)
            .expect("list");
        assert_eq!(events.len(), 0);
    }
}
