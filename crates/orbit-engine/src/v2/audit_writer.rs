use std::sync::Mutex;

use chrono::Utc;
use orbit_agent::loop_engine::audit::{AuditSink, LoopAuditEvent};
use orbit_types::v2::{
    AUDIT_ENVELOPE_SCHEMA_VERSION, V2AuditEnvelope, V2AuditEvent, V2AuditEventKind,
};
use thiserror::Error;

/// Writes §7 v2 audit envelope events. Nests the existing loop-engine events
/// underneath an Activity event via `parent_event_id` so the whole tree
/// (Run → Step → Activity → http.*/tool.call.*) is traversable by ID.
///
/// This writer owns the run_id / agent_identity context and emits events both
/// as structured JSON (for orbit-audit consumers) and as an inner loop sink
/// passthrough (so loop-level http.* and tool.call.* events continue to flow
/// through the existing JSONL path).
pub struct V2AuditWriter {
    run_id: String,
    agent_identity: String,
    inner: Box<dyn AuditSink>,
    events: Mutex<Vec<V2AuditEvent>>,
    event_counter: Mutex<u64>,
    parent_stack: Mutex<Vec<String>>,
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("audit writer mutex poisoned")]
    Poisoned,
}

impl V2AuditWriter {
    pub fn new(
        run_id: impl Into<String>,
        agent_identity: impl Into<String>,
        inner: Box<dyn AuditSink>,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            agent_identity: agent_identity.into(),
            inner,
            events: Mutex::new(Vec::new()),
            event_counter: Mutex::new(0),
            parent_stack: Mutex::new(Vec::new()),
        }
    }

    /// Emit a v2 envelope event of the given kind. Returns the event_id so
    /// callers can use it as a parent for nested events.
    pub fn emit(&self, kind: V2AuditEventKind) -> Result<String, WriteError> {
        let event_id = self.next_event_id()?;
        let parent_event_id = self
            .parent_stack
            .lock()
            .map_err(|_| WriteError::Poisoned)?
            .last()
            .cloned();
        let event_type = event_type_of(&kind).to_string();
        let envelope = V2AuditEnvelope {
            schema_version: AUDIT_ENVELOPE_SCHEMA_VERSION,
            event_type,
            event_id: event_id.clone(),
            ts: Utc::now(),
            run_id: self.run_id.clone(),
            agent_identity: self.agent_identity.clone(),
            parent_event_id,
        };
        let event = V2AuditEvent { envelope, kind };
        self.events
            .lock()
            .map_err(|_| WriteError::Poisoned)?
            .push(event);
        Ok(event_id)
    }

    /// Push a parent context so subsequent events nest beneath it.
    pub fn push_parent(&self, event_id: String) -> Result<(), WriteError> {
        self.parent_stack
            .lock()
            .map_err(|_| WriteError::Poisoned)?
            .push(event_id);
        Ok(())
    }

    /// Pop the most recent parent context.
    pub fn pop_parent(&self) -> Result<Option<String>, WriteError> {
        Ok(self
            .parent_stack
            .lock()
            .map_err(|_| WriteError::Poisoned)?
            .pop())
    }

    /// Snapshot of emitted events (for smoke verification).
    pub fn events_snapshot(&self) -> Result<Vec<V2AuditEvent>, WriteError> {
        Ok(self
            .events
            .lock()
            .map_err(|_| WriteError::Poisoned)?
            .clone())
    }

    /// Access to the inner loop-level sink for the loop engine to emit
    /// http.*/tool.call.* events through.
    pub fn inner_sink(&self) -> &dyn AuditSink {
        self.inner.as_ref()
    }

    /// Proxy: write a blob via the inner sink (sha256-based, per §7.4 / §12 Q11).
    pub fn write_blob(&self, content: &[u8]) -> String {
        self.inner.write_blob(content)
    }

    /// Proxy: emit a loop-level event through the inner sink.
    pub fn emit_loop_event(&self, event: &LoopAuditEvent) {
        self.inner.emit(event);
    }

    fn next_event_id(&self) -> Result<String, WriteError> {
        let mut counter = self
            .event_counter
            .lock()
            .map_err(|_| WriteError::Poisoned)?;
        *counter += 1;
        Ok(format!("v2evt-{}-{:08x}", self.run_id, *counter))
    }
}

fn event_type_of(kind: &V2AuditEventKind) -> &'static str {
    match kind {
        V2AuditEventKind::RunStarted { .. } => "run.started",
        V2AuditEventKind::RunFinished { .. } => "run.finished",
        V2AuditEventKind::StepStarted { .. } => "step.started",
        V2AuditEventKind::StepFinished { .. } => "step.finished",
        V2AuditEventKind::ActivityStarted { .. } => "activity.started",
        V2AuditEventKind::ActivityFinished { .. } => "activity.finished",
        V2AuditEventKind::ToolDenied { .. } => "tool.denied",
    }
}
