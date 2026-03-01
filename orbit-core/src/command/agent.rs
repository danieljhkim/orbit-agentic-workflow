use chrono::Utc;
use orbit_types::{
    AgentSession, AgentSessionStatus, AgentToolCall, IdentityRole, OrbitError, OrbitEvent, Role,
};
use serde_json::json;

use crate::OrbitRuntime;
use crate::agent::context::{compose_agent_context, parse_planned_tool_calls};

#[derive(Debug, Clone)]
pub struct AgentRunResult {
    pub session_id: String,
    pub task_id: String,
    pub tool_calls_executed: usize,
    pub status: AgentSessionStatus,
}

#[derive(Debug, Clone, Default)]
pub struct AgentRunOptions {
    pub approve_on_verbal: bool,
    pub approved_by: Option<String>,
    pub approval_note: Option<String>,
    pub identity_id: Option<String>,
}

impl OrbitRuntime {
    pub fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSession>, OrbitError> {
        self.context.store.get_agent_session(session_id)
    }

    pub fn run_agent_task(&self, task_id: &str) -> Result<AgentRunResult, OrbitError> {
        self.run_agent_task_with_options(task_id, AgentRunOptions::default())
    }

    pub fn run_agent_task_with_options(
        &self,
        task_id: &str,
        options: AgentRunOptions,
    ) -> Result<AgentRunResult, OrbitError> {
        let mut task = self.get_task(task_id)?;
        let identity_id = options
            .identity_id
            .clone()
            .or_else(|| task.identity_id.clone());
        let resolved_identity = identity_id
            .as_deref()
            .map(|id| self.resolve_identity(id))
            .transpose()?;
        let identity_block = resolved_identity
            .as_ref()
            .map(|identity| self.compile_identity_block(identity));

        let skills = Vec::new();
        let session_id = format!(
            "session-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        );

        let composed = match compose_agent_context(
            self,
            &task,
            &skills,
            Role::Agent,
            identity_block.as_deref(),
        ) {
            Ok(composed) => composed,
            Err(err) => {
                let _ = self.with_mutation(|_| {
                    Ok((
                        (),
                        OrbitEvent::AgentSessionCompleted {
                            session_id: session_id.clone(),
                            task_id: task.id.clone(),
                            status: "failed".to_string(),
                        },
                    ))
                });
                return Err(err);
            }
        };

        let planned_calls = match parse_planned_tool_calls(&task.instructions) {
            Ok(calls) => calls,
            Err(err) => {
                let _ = self.with_mutation(|_| {
                    Ok((
                        (),
                        OrbitEvent::AgentSessionCompleted {
                            session_id: session_id.clone(),
                            task_id: task.id.clone(),
                            status: "failed".to_string(),
                        },
                    ))
                });
                return Err(err);
            }
        };

        let now = Utc::now();
        let mut executed_calls: Vec<AgentToolCall> = Vec::new();
        let skill_names = composed.skill_names.clone();
        let mut session_outcome = "running".to_string();

        let session = AgentSession {
            session_id: session_id.clone(),
            task_id: task.id.clone(),
            identity_id: resolved_identity.as_ref().map(|value| value.id.clone()),
            identity_name: resolved_identity.as_ref().map(|value| value.name.clone()),
            identity_role: resolved_identity.as_ref().map(|value| value.role),
            identity_block: identity_block.clone(),
            skill_names: skill_names.clone(),
            composed_context_hash: composed.composed_context_hash.clone(),
            effective_allowed_tools: composed.effective_allowed_tools.clone(),
            tool_calls: vec![],
            outcome: session_outcome.clone(),
            status: AgentSessionStatus::Running,
            created_at: now,
            updated_at: now,
        };

        self.with_mutation(|tx| {
            tx.insert_agent_session(&session)?;
            Ok((
                (),
                OrbitEvent::AgentSessionStarted {
                    session_id: session_id.clone(),
                    task_id: task.id.clone(),
                    identity_id: session.identity_id.clone(),
                    identity_name: session.identity_name.clone(),
                    identity_role: session.identity_role.map(|v| v.to_string()),
                    identity_block: session.identity_block.clone(),
                    skill_names: skill_names.clone(),
                    composed_context_hash: composed.composed_context_hash.clone(),
                    effective_allowed_tools: composed.effective_allowed_tools.clone(),
                },
            ))
        })?;

        if self.context.task_approval_required_for_agent && task.approved_at.is_none() {
            if options.approve_on_verbal {
                task = if resolved_identity
                    .as_ref()
                    .is_some_and(|identity| identity.role == IdentityRole::Leader)
                {
                    let approval_note = options.approval_note.or_else(|| {
                        Some(
                            "Approved on explicit verbal confirmation from user by leader identity"
                                .to_string(),
                        )
                    });
                    self.approve_task_from_session(task_id, &session_id, approval_note)?
                } else {
                    let approved_by = options.approved_by.unwrap_or_else(|| "agent".to_string());
                    let approval_note = options.approval_note.or_else(|| {
                        Some("Approved on explicit verbal confirmation from user".to_string())
                    });
                    self.approve_task(task_id, &approved_by, approval_note)?
                };
            } else {
                self.finish_agent_session(
                    &session_id,
                    &task.id,
                    &executed_calls,
                    "task requires approval",
                    AgentSessionStatus::Failed,
                )?;
                return Err(OrbitError::TaskApprovalRequired(format!(
                    "task '{task_id}' is not approved; run `orbit task approve {task_id}` or `orbit agent run --task {task_id} --approve-on-verbal`"
                )));
            }
        }

        for mut planned in planned_calls {
            if !composed.effective_allowed_tools.contains(&planned.name) {
                planned.success = false;
                planned.output = Some(json!({
                    "error": format!("tool '{}' not permitted by effective allowlist", planned.name)
                }));
                executed_calls.push(planned.clone());
                let session_status = AgentSessionStatus::Failed;
                session_outcome = "tool not permitted".to_string();

                self.record_agent_tool_call(
                    &AgentSessionUpdate {
                        session_id: &session_id,
                        task_id: &task.id,
                        skill_names: &skill_names,
                        all_calls: &executed_calls,
                        outcome: &session_outcome,
                        status: session_status.clone(),
                    },
                    &planned,
                )?;
                self.finish_agent_session(
                    &session_id,
                    &task.id,
                    &executed_calls,
                    &session_outcome,
                    session_status.clone(),
                )?;

                return Err(OrbitError::PolicyDenied(format!(
                    "tool '{}' not permitted by effective allowlist",
                    planned.name
                )));
            }

            match self.run_tool_with_role(&planned.name, planned.input.clone(), composed.role) {
                Ok(output) => {
                    planned.success = true;
                    planned.output = Some(output);
                    executed_calls.push(planned.clone());
                    self.record_agent_tool_call(
                        &AgentSessionUpdate {
                            session_id: &session_id,
                            task_id: &task.id,
                            skill_names: &skill_names,
                            all_calls: &executed_calls,
                            outcome: "running",
                            status: AgentSessionStatus::Running,
                        },
                        &planned,
                    )?;
                }
                Err(err) => {
                    planned.success = false;
                    planned.output = Some(json!({ "error": err.to_string() }));
                    executed_calls.push(planned.clone());
                    let session_status = AgentSessionStatus::Failed;
                    session_outcome = err.to_string();
                    self.record_agent_tool_call(
                        &AgentSessionUpdate {
                            session_id: &session_id,
                            task_id: &task.id,
                            skill_names: &skill_names,
                            all_calls: &executed_calls,
                            outcome: &session_outcome,
                            status: session_status.clone(),
                        },
                        &planned,
                    )?;
                    self.finish_agent_session(
                        &session_id,
                        &task.id,
                        &executed_calls,
                        &session_outcome,
                        session_status.clone(),
                    )?;
                    return Err(err);
                }
            }
        }

        let session_status = AgentSessionStatus::Completed;
        session_outcome = "completed".to_string();
        self.finish_agent_session(
            &session_id,
            &task.id,
            &executed_calls,
            &session_outcome,
            session_status.clone(),
        )?;

        Ok(AgentRunResult {
            session_id,
            task_id: task.id,
            tool_calls_executed: executed_calls.len(),
            status: session_status,
        })
    }

    fn record_agent_tool_call(
        &self,
        update: &AgentSessionUpdate<'_>,
        call: &AgentToolCall,
    ) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            tx.update_agent_session(
                update.session_id,
                update.all_calls,
                update.outcome,
                update.status.clone(),
            )?;
            Ok((
                (),
                OrbitEvent::AgentToolCall {
                    session_id: update.session_id.to_string(),
                    task_id: update.task_id.to_string(),
                    skill_names: update.skill_names.to_vec(),
                    tool_name: call.name.clone(),
                    input: call.input.clone(),
                    output: call.output.clone(),
                    success: call.success,
                },
            ))
        })
    }

    fn finish_agent_session(
        &self,
        session_id: &str,
        task_id: &str,
        all_calls: &[AgentToolCall],
        outcome: &str,
        status: AgentSessionStatus,
    ) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            tx.update_agent_session(session_id, all_calls, outcome, status.clone())?;
            Ok((
                (),
                OrbitEvent::AgentSessionCompleted {
                    session_id: session_id.to_string(),
                    task_id: task_id.to_string(),
                    status: status_to_text(&status).to_string(),
                },
            ))
        })
    }
}

fn status_to_text(status: &AgentSessionStatus) -> &'static str {
    match status {
        AgentSessionStatus::Running => "running",
        AgentSessionStatus::Completed => "completed",
        AgentSessionStatus::Failed => "failed",
    }
}

struct AgentSessionUpdate<'a> {
    session_id: &'a str,
    task_id: &'a str,
    skill_names: &'a [String],
    all_calls: &'a [AgentToolCall],
    outcome: &'a str,
    status: AgentSessionStatus,
}
