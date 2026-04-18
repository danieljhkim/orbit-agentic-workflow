use orbit_types::v2::ActivityV2Spec;
use serde_json::Value;
use thiserror::Error;

use super::audit_writer::V2AuditWriter;

/// Input bundle for a single v2 activity dispatch.
pub struct V2DispatchInput<'a> {
    pub activity_name: &'a str,
    pub spec: &'a ActivityV2Spec,
    pub input: Value,
    pub audit: &'a V2AuditWriter,
}

/// Outcome of a v2 dispatch attempt. Kept separate from v1's AttemptOutcome
/// to avoid coupling v2 callers to the v1 engine context.
#[derive(Debug, Clone)]
pub struct DispatchOutcome {
    pub success: bool,
    pub output: Value,
    pub message: Option<String>,
}

#[derive(Debug, Error)]
pub enum DispatchError {
    /// agent_loop runtime requires Session/Transport/ToolRegistry to be wired
    /// at the caller (orbit-core). Until that wiring is in place, v2
    /// agent_loop dispatches return this variant instead of panicking.
    #[error(
        "agent_loop dispatch requires loop-engine infrastructure not yet wired into v2 dispatcher"
    )]
    AgentLoopInfraNotWired,

    #[error("deterministic action not registered: {0}")]
    DeterministicActionNotRegistered(String),

    #[error("shell program `{0}` not in allowed_programs")]
    ShellProgramNotAllowed(String),

    #[error("shell program spawn failed: {0}")]
    ShellSpawnFailed(String),

    #[error("unknown activity type (should be unreachable): {0}")]
    UnknownType(String),
}

/// Dispatch a v2 activity by type. This is the v2-side analogue of
/// `run_activity_direct` for the v1 path.
///
/// Phase 2 scope: this function handles the type-match and emits the §7
/// activity.started / activity.finished envelope events. The per-type runners
/// are invoked as thin helpers; agent_loop specifically requires loop-engine
/// infrastructure wired at the caller (orbit-core) and returns a structured
/// `DispatchError::AgentLoopInfraNotWired` when the required dependencies are
/// not present. Phase 4 will add the orbit-core wiring; Phase 2 proves the
/// dispatch shape compiles and returns an error rather than panicking.
pub fn dispatch_v2_activity(input: V2DispatchInput<'_>) -> Result<DispatchOutcome, DispatchError> {
    let activity_type = match input.spec {
        ActivityV2Spec::AgentLoop(_) => "agent_loop",
        ActivityV2Spec::Deterministic(_) => "deterministic",
        ActivityV2Spec::Shell(_) => "shell",
    };

    let activity_event_id = input
        .audit
        .emit(orbit_types::v2::V2AuditEventKind::ActivityStarted {
            activity_name: input.activity_name.to_string(),
            activity_type: activity_type.to_string(),
        })
        .map_err(|err| DispatchError::ShellSpawnFailed(format!("audit: {err:?}")))?;
    let _ = input.audit.push_parent(activity_event_id);

    let result = match input.spec {
        ActivityV2Spec::AgentLoop(_spec) => Err(DispatchError::AgentLoopInfraNotWired),
        ActivityV2Spec::Deterministic(spec) => run_deterministic(spec, input.input.clone()),
        ActivityV2Spec::Shell(spec) => run_shell(spec, input.input.clone()),
    };

    let _ = input.audit.pop_parent();
    let outcome_str = match &result {
        Ok(o) if o.success => "success",
        Ok(_) => "failed",
        Err(_) => "error",
    };
    let _ = input
        .audit
        .emit(orbit_types::v2::V2AuditEventKind::ActivityFinished {
            activity_name: input.activity_name.to_string(),
            outcome: outcome_str.to_string(),
        });

    result
}

fn run_deterministic(
    spec: &orbit_types::v2::DeterministicSpec,
    _input: Value,
) -> Result<DispatchOutcome, DispatchError> {
    // Phase 2: the deterministic runtime path dispatches by registered action
    // name into the existing ActivityExecutorRegistry. Full runtime wiring
    // (ExecutorHost, ExecutionContext construction) happens at the caller —
    // this runner is intentionally thin and compiles without side effects.
    //
    // Returning a non-success outcome with the action name is the structural
    // placeholder; callers wire the registry lookup at Phase 4 cutover.
    Ok(DispatchOutcome {
        success: false,
        output: serde_json::json!({"action": spec.action, "status": "infra_not_wired"}),
        message: Some(format!(
            "deterministic action `{}` dispatch not yet wired into v2 runner",
            spec.action
        )),
    })
}

fn run_shell(
    spec: &orbit_types::v2::ShellSpec,
    _input: Value,
) -> Result<DispatchOutcome, DispatchError> {
    if !spec.allowed_programs.contains(&spec.program) {
        return Err(DispatchError::ShellProgramNotAllowed(spec.program.clone()));
    }
    // Phase 2: the shell runner validates the allowlist and returns a
    // structural success. Full runtime execution (spawn via orbit-exec,
    // capture exit code, apply expected_exit_codes policy) is wired by the
    // caller at Phase 4 cutover — this runner compiles without spawning
    // processes from the dispatcher itself.
    Ok(DispatchOutcome {
        success: true,
        output: serde_json::json!({
            "program": spec.program,
            "args": spec.args,
            "status": "allowlist_validated",
        }),
        message: None,
    })
}
