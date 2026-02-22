use std::time::Instant;

use orbit_core::{AuditEventInsertParams, OrbitError, OrbitRuntime};
use orbit_types::AuditEventStatus;

use crate::command::Commands;

pub struct CommandMeta {
    pub command: String,
    pub subcommand: Option<String>,
    pub tool_name: Option<String>,
    pub target_type: Option<String>,
    pub target_id: Option<String>,
    pub role: String,
    pub arguments_json: Option<String>,
}

pub fn execute_with_audit<F>(
    runtime: &OrbitRuntime,
    meta: CommandMeta,
    f: F,
) -> Result<(), OrbitError>
where
    F: FnOnce() -> Result<(), OrbitError>,
{
    let start = Instant::now();
    let result = f();
    let duration_ms = start.elapsed().as_millis() as i64;

    let (status, exit_code, error_message) = match &result {
        Ok(()) => (AuditEventStatus::Success, 0, None),
        Err(OrbitError::PolicyDenied(msg)) => (AuditEventStatus::Denied, 1, Some(msg.to_string())),
        Err(err) => (AuditEventStatus::Failure, 1, Some(err.to_string())),
    };

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let execution_id = format!("exec-{nanos}");

    let working_directory = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string());

    let host = std::env::var("HOSTNAME").ok();
    let pid = std::process::id();

    let params = AuditEventInsertParams {
        execution_id,
        command: meta.command,
        subcommand: meta.subcommand,
        tool_name: meta.tool_name,
        target_type: meta.target_type,
        target_id: meta.target_id,
        role: meta.role,
        status,
        exit_code,
        duration_ms,
        working_directory,
        arguments_json: meta.arguments_json,
        stdout_truncated: None,
        stderr_truncated: None,
        error_message,
        host,
        pid,
        session_id: None,
    };

    if let Err(audit_err) = runtime.record_audit_event(&params) {
        eprintln!("warning: failed to write audit event: {audit_err}");
    }

    result
}

pub fn extract_command_meta(cmd: &Commands) -> CommandMeta {
    match cmd {
        Commands::Tool(tool_cmd) => {
            use crate::command::tool::ToolSubcommand;
            let (sub, tool_name, target_type, target_id) = match &tool_cmd.command {
                ToolSubcommand::Run(args) => (
                    "run",
                    Some(args.name.clone()),
                    Some("tool".to_string()),
                    Some(args.name.clone()),
                ),
                ToolSubcommand::List(_) => ("list", None, None, None),
                ToolSubcommand::Show(args) => (
                    "show",
                    Some(args.name.clone()),
                    Some("tool".to_string()),
                    Some(args.name.clone()),
                ),
                ToolSubcommand::Add(args) => (
                    "add",
                    args.name.clone(),
                    Some("tool".to_string()),
                    args.name.clone(),
                ),
                ToolSubcommand::Remove(args) => (
                    "remove",
                    Some(args.name.clone()),
                    Some("tool".to_string()),
                    Some(args.name.clone()),
                ),
                ToolSubcommand::Enable(args) => (
                    "enable",
                    Some(args.name.clone()),
                    Some("tool".to_string()),
                    Some(args.name.clone()),
                ),
                ToolSubcommand::Disable(args) => (
                    "disable",
                    Some(args.name.clone()),
                    Some("tool".to_string()),
                    Some(args.name.clone()),
                ),
                ToolSubcommand::Doctor => ("doctor", None, None, None),
            };
            CommandMeta {
                command: "tool".to_string(),
                subcommand: Some(sub.to_string()),
                tool_name,
                target_type,
                target_id,
                role: "admin".to_string(),
                arguments_json: None,
            }
        }
        Commands::Task(task_cmd) => {
            use crate::command::task::TaskSubcommand;
            let (sub, target_type, target_id) = match &task_cmd.command {
                TaskSubcommand::Add(_) => ("add", Some("task"), None),
                TaskSubcommand::List(_) => ("list", None, None),
                TaskSubcommand::Show(args) => ("show", Some("task"), Some(args.id.as_str())),
                TaskSubcommand::Update(args) => ("update", Some("task"), Some(args.id.as_str())),
                TaskSubcommand::Close(args) => ("close", Some("task"), Some(args.id.as_str())),
                TaskSubcommand::Reopen(args) => ("reopen", Some("task"), Some(args.id.as_str())),
                TaskSubcommand::Delete(args) => ("delete", Some("task"), Some(args.id.as_str())),
                TaskSubcommand::Search(_) => ("search", None, None),
            };
            CommandMeta {
                command: "task".to_string(),
                subcommand: Some(sub.to_string()),
                tool_name: None,
                target_type: target_type.map(String::from),
                target_id: target_id.map(String::from),
                role: "admin".to_string(),
                arguments_json: None,
            }
        }
        Commands::Job(job_cmd) => {
            use crate::command::job::JobSubcommand;
            let (sub, target_id) = match &job_cmd.command {
                JobSubcommand::Add(_) => ("add", None),
                JobSubcommand::List(_) => ("list", None),
                JobSubcommand::Show(args) => ("show", Some(args.job_id.as_str())),
                JobSubcommand::Run(args) => ("run", Some(args.job_id.as_str())),
                JobSubcommand::Pause(args) => ("pause", Some(args.job_id.as_str())),
                JobSubcommand::Resume(args) => ("resume", Some(args.job_id.as_str())),
                JobSubcommand::Cancel(args) => ("cancel", Some(args.job_id.as_str())),
                JobSubcommand::History(args) => ("history", Some(args.job_id.as_str())),
                JobSubcommand::Delete(args) => ("delete", Some(args.job_id.as_str())),
            };
            CommandMeta {
                command: "job".to_string(),
                subcommand: Some(sub.to_string()),
                tool_name: None,
                target_type: Some("job".to_string()),
                target_id: target_id.map(String::from),
                role: "admin".to_string(),
                arguments_json: None,
            }
        }
        Commands::Agent(_) => CommandMeta {
            command: "agent".to_string(),
            subcommand: Some("run".to_string()),
            tool_name: None,
            target_type: None,
            target_id: None,
            role: "admin".to_string(),
            arguments_json: None,
        },
        Commands::Entry(_) => CommandMeta {
            command: "entry".to_string(),
            subcommand: None,
            tool_name: None,
            target_type: None,
            target_id: None,
            role: "admin".to_string(),
            arguments_json: None,
        },
        Commands::Skill(_) => CommandMeta {
            command: "skill".to_string(),
            subcommand: None,
            tool_name: None,
            target_type: None,
            target_id: None,
            role: "admin".to_string(),
            arguments_json: None,
        },
        Commands::Watch(_) => CommandMeta {
            command: "watch".to_string(),
            subcommand: None,
            tool_name: None,
            target_type: None,
            target_id: None,
            role: "admin".to_string(),
            arguments_json: None,
        },
        Commands::Audit(_) => unreachable!("audit commands should not be audited"),
    }
}
