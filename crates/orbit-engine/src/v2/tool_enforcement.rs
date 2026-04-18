use std::sync::{Arc, Mutex};

use orbit_agent::loop_engine::audit::{AuditSink, LoopAuditEvent};
use orbit_types::v2::{V2AuditEventKind, tool_allowed};

use super::audit_writer::V2AuditWriter;

/// Decision emitted when enforcement fires.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnforcementDecision {
    Allowed,
    Denied { tool_name: String, reason: String },
}

/// AuditSink wrapper that enforces a tool allowlist at the Orbit layer.
///
/// Usage: Build the inner sink (e.g. JsonlFileSink / InMemorySink), wrap it
/// with an EnforcedAuditSink at construction, and pass the wrapper into
/// AgentLoop::run via the audit parameter. The wrapper intercepts
/// ToolCallRequested events, checks the name against the allowlist, and (on
/// deny) substitutes a PolicyDenial event and signals the caller to
/// terminate.
pub struct EnforcedAuditSink {
    inner: Arc<dyn AuditSink>,
    allowlist: Vec<String>,
    writer: Arc<V2AuditWriter>,
    tripped: Mutex<Option<EnforcementDecision>>,
}

impl EnforcedAuditSink {
    pub fn new(
        inner: Arc<dyn AuditSink>,
        allowlist: Vec<String>,
        writer: Arc<V2AuditWriter>,
    ) -> Self {
        Self {
            inner,
            allowlist,
            writer,
            tripped: Mutex::new(None),
        }
    }

    pub fn tripped(&self) -> Option<EnforcementDecision> {
        self.tripped.lock().expect("tripped mutex").clone()
    }
}

impl AuditSink for EnforcedAuditSink {
    fn emit(&self, event: &LoopAuditEvent) {
        if let LoopAuditEvent::ToolCallRequested { tool_name, .. } = event
            && !tool_allowed(tool_name, &self.allowlist)
        {
            let reason = format!("tool `{tool_name}` not in allowlist");
            // Emit a §7 tool.denied envelope event.
            let _ = self.writer.emit(V2AuditEventKind::ToolDenied {
                tool_name: tool_name.clone(),
                reason: reason.clone(),
            });
            // Preserve the original event in the loop stream too, so a
            // review can see the inbound request that was denied.
            self.inner.emit(event);
            // Additionally emit a PolicyDenial on the inner loop sink so
            // the loop-level JSONL records the enforcement decision.
            let denial = LoopAuditEvent::PolicyDenial {
                ts: chrono::Utc::now(),
                run_id: String::new(),
                session_id: String::new(),
                iteration: 0,
                tool_name: tool_name.clone(),
                reason: reason.clone(),
            };
            self.inner.emit(&denial);
            *self.tripped.lock().expect("tripped mutex") = Some(EnforcementDecision::Denied {
                tool_name: tool_name.clone(),
                reason,
            });
            return;
        }
        self.inner.emit(event);
    }

    fn write_blob(&self, content: &[u8]) -> String {
        self.inner.write_blob(content)
    }
}
