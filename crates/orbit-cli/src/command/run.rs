use std::collections::HashMap;
use std::fs;

use clap::{Args, Subcommand};
use orbit_common::utility::blob_store::BlobStore;
use orbit_core::command::job_run::JobRunListParams;
use orbit_core::{JobRun, JobRunStep, OrbitError, OrbitRuntime};
use serde_json::{Value, json};

use crate::command::Execute;
use crate::command::job::{job_run_to_json, summarize_error_message};
use crate::command::{duel, job, ship};

const DEFAULT_HISTORY_LIMIT: usize = 50;

const RUN_AFTER_HELP: &str = "\
Workflow entrypoints:
  orbit run ship <task_id> ...
  orbit run ship-auto
  orbit run duel-plan <task_id>
  orbit run job <job_id> [--input key=value] [--json] [--debug]

Direct form:
  orbit run <job_id> [--input key=value] [--json] [--debug]
    Equivalent to `orbit run job <job_id>`.

Run history:
  orbit run history [--limit 50]
  orbit run history -j <job_id>
  orbit run show [run_id] [-s step_id] [--json]
  orbit run logs [run_id] [-s step_id] [--json]
";

#[derive(Args)]
#[command(
    about = "Run a job workflow (supports run ship / ship-auto / duel-plan / job / run <id>)",
    arg_required_else_help = true,
    args_conflicts_with_subcommands = true,
    override_usage = "orbit run <COMMAND>\n       orbit run <JOB_ID> [OPTIONS]",
    after_help = RUN_AFTER_HELP
)]
pub struct RunCommand {
    #[command(subcommand)]
    pub command: Option<RunSubcommand>,

    #[command(flatten)]
    pub positional: PositionalJobArgs,
}

impl Execute for RunCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self.command {
            Some(command) => command.execute(runtime),
            None => execute_positional_job(self.positional, runtime),
        }
    }
}

#[derive(Subcommand)]
pub enum RunSubcommand {
    /// Ship explicitly selected tasks through the task pipeline
    Ship(ship::ShipCommand),
    /// Auto-select backlog tasks and ship them through the task pipeline
    #[command(name = "ship-auto")]
    ShipAuto(ship::ShipAutoCommand),
    /// Run a planning duel for one task
    #[command(name = "duel-plan")]
    DuelPlan(duel::DuelPlanCommand),
    /// Show recent job runs, optionally filtered to one job
    History(RunHistoryArgs),
    /// Show structured state and step summary for a job run
    Show(RunShowArgs),
    /// Print raw stdout/stderr captured for a job run
    Logs(RunLogsArgs),
    /// Run an arbitrary job by ID
    Job(job::JobRunArgs),
}

impl Execute for RunSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            RunSubcommand::Ship(command) => command.execute(runtime),
            RunSubcommand::ShipAuto(command) => command.execute(runtime),
            RunSubcommand::DuelPlan(command) => command.execute(runtime),
            RunSubcommand::History(command) => command.execute(runtime),
            RunSubcommand::Show(command) => command.execute(runtime),
            RunSubcommand::Logs(command) => command.execute(runtime),
            RunSubcommand::Job(command) => command.execute(runtime),
        }
    }
}

#[derive(Args)]
#[command(
    after_help = "JSON shape: {\"runs\":[<job-run>]}\nExamples:\n  orbit run history\n  orbit run history -j task_local_pipeline --limit 20\n  orbit run history --json"
)]
pub struct RunHistoryArgs {
    /// Filter to one job ID
    #[arg(short = 'j', long = "job")]
    pub job_id: Option<String>,

    /// Maximum number of runs to show
    #[arg(long, default_value_t = DEFAULT_HISTORY_LIMIT)]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for RunHistoryArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        print_run_history(runtime, self.job_id.as_deref(), Some(self.limit), self.json)
    }
}

#[derive(Args)]
#[command(
    after_help = "JSON shape: {\"run\":<job-run>,\"pipeline_state\":<state|null>} or {\"run_id\":...,\"job_id\":...,\"step\":<step>,\"step_output\":<json|null>} with -s.\nExamples:\n  orbit run show\n  orbit run show jrun-20260426-0631\n  orbit run show jrun-20260426-0631 -s implement_one --json"
)]
pub struct RunShowArgs {
    /// Run ID to inspect. Defaults to the most recently scheduled run globally.
    pub run_id: Option<String>,

    /// Show a single step by target/step ID
    #[arg(short = 's', long = "step")]
    pub step_id: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for RunShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        print_run_show(
            runtime,
            self.run_id.as_deref(),
            self.step_id.as_deref(),
            self.json,
        )
    }
}

#[derive(Args)]
#[command(
    after_help = "JSON shape: {\"run_id\":\"...\",\"job_id\":\"...\",\"records\":[{\"step_id\":...,\"stdout_blob_ref\":...,\"stderr_blob_ref\":...,\"stdout\":\"...\",\"stderr\":\"...\"}]}\nExamples:\n  orbit run logs\n  orbit run logs jrun-20260426-0631\n  orbit run logs jrun-20260426-0631 -s implement_one --json"
)]
pub struct RunLogsArgs {
    /// Run ID to inspect. Defaults to the most recently scheduled run globally.
    pub run_id: Option<String>,

    /// Show raw logs for a single step ID
    #[arg(short = 's', long = "step")]
    pub step_id: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for RunLogsArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        print_run_logs(
            runtime,
            self.run_id.as_deref(),
            self.step_id.as_deref(),
            self.json,
        )
    }
}

#[derive(Args, Default)]
pub struct PositionalJobArgs {
    /// Run the named job directly (equivalent to `orbit run job <JOB_ID>`)
    pub job_id: Option<String>,

    /// Input key=value pairs passed to all job steps (repeatable)
    #[arg(long)]
    pub input: Vec<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Stream agent stderr to the terminal and tee stdout live for debugging
    #[arg(long)]
    pub debug: bool,
}

fn execute_positional_job(
    args: PositionalJobArgs,
    runtime: &OrbitRuntime,
) -> Result<(), OrbitError> {
    let Some(job_id) = args.job_id else {
        return Err(OrbitError::InvalidInput(
            "`orbit run` expects a workflow subcommand or job ID".to_string(),
        ));
    };

    ensure_positional_job_exists(runtime, &job_id)?;

    job::JobRunArgs {
        job_id,
        input: args.input,
        backend: None,
        json: args.json,
        debug: args.debug,
    }
    .execute(runtime)
}

fn ensure_positional_job_exists(runtime: &OrbitRuntime, job_id: &str) -> Result<(), OrbitError> {
    match runtime.show_job_catalog_entry(job_id) {
        Ok(_) => Ok(()),
        Err(OrbitError::JobNotFound(_)) => Err(OrbitError::InvalidInput(format!(
            "unknown `orbit run` target `{job_id}`\navailable subcommands: ship, ship-auto, duel-plan, job\ntip: use `orbit job list` to discover valid job ids"
        ))),
        Err(error) => Err(error),
    }
}

pub(crate) fn print_run_history(
    runtime: &OrbitRuntime,
    job_id: Option<&str>,
    limit: Option<usize>,
    json_output: bool,
) -> Result<(), OrbitError> {
    let runs = match job_id {
        Some(job_id) => runtime.list_job_runs(JobRunListParams {
            job_id: Some(job_id.to_string()),
            limit,
            ..Default::default()
        })?,
        None => runtime.list_job_runs(JobRunListParams {
            limit,
            ..Default::default()
        })?,
    };

    if json_output {
        let values = runs.iter().map(job_run_to_json).collect::<Vec<_>>();
        return crate::output::json::print_pretty(&json!({ "runs": values }));
    }

    let include_job_id = job_id.is_none();
    let headers = if include_job_id {
        vec![
            "RUN_ID",
            "JOB_ID",
            "ATTEMPT",
            "STATE",
            "STARTED_AT",
            "FINISHED_AT",
            "ERROR_CODE",
            "ERROR_MESSAGE",
        ]
    } else {
        vec![
            "RUN_ID",
            "ATTEMPT",
            "STATE",
            "STARTED_AT",
            "FINISHED_AT",
            "ERROR_CODE",
            "ERROR_MESSAGE",
        ]
    };
    let mut table = crate::output::table::build_table(&headers);
    for run in &runs {
        use comfy_table::Cell;
        let last = run.steps.last();
        let mut row = vec![Cell::new(&run.run_id)];
        if include_job_id {
            row.push(Cell::new(&run.job_id));
        }
        row.extend([
            Cell::new(run.attempt.to_string()),
            crate::output::color::job_state_color_cell(&run.state.to_string()),
            Cell::new(format_timestamp(run.started_at)),
            Cell::new(format_timestamp(run.finished_at)),
            Cell::new(last.and_then(|s| s.error_code.as_deref()).unwrap_or("-")),
            Cell::new(summarize_error_message(
                last.and_then(|s| s.error_message.as_deref()),
            )),
        ]);
        table.add_row(row);
    }
    println!("{table}");
    Ok(())
}

pub(crate) fn print_run_show(
    runtime: &OrbitRuntime,
    run_id: Option<&str>,
    step_id: Option<&str>,
    json_output: bool,
) -> Result<(), OrbitError> {
    let run = resolve_run(runtime, run_id)?;
    let state = runtime.read_run_state(&run.run_id)?;

    if let Some(step_id) = step_id {
        let step = find_run_step(&run, step_id)?;
        let step_output = state
            .as_ref()
            .and_then(|state| state.step_outputs.get(&step.step_index))
            .cloned();
        return print_step_record(&run, step, step_output, json_output);
    }

    if json_output {
        return crate::output::json::print_pretty(&json!({
            "run": job_run_to_json(&run),
            "pipeline_state": state,
        }));
    }

    print_run_header(&run);
    if let Some(state) = &state {
        println!(
            "{} iteration={} step_outputs={} updated_at={}",
            crate::output::color::bold("Pipeline:"),
            state.iteration,
            state.step_outputs.len(),
            state.updated_at.to_rfc3339(),
        );
    }
    println!();
    let steps = run.steps.iter().collect::<Vec<_>>();
    print_step_summary_table(&steps)
}

pub(crate) fn print_run_logs(
    runtime: &OrbitRuntime,
    run_id: Option<&str>,
    step_id: Option<&str>,
    json_output: bool,
) -> Result<(), OrbitError> {
    let run = resolve_run(runtime, run_id)?;
    let records = collect_raw_log_records(runtime, &run, step_id)?;

    if json_output {
        return crate::output::json::print_pretty(&json!({
            "run_id": run.run_id,
            "job_id": run.job_id,
            "records": records.iter().map(raw_log_record_to_json).collect::<Vec<_>>(),
        }));
    }

    if records.is_empty() {
        println!("No raw stdout/stderr blobs recorded.");
        return Ok(());
    }

    for record in &records {
        print!("{}", record.stdout);
        eprint!("{}", record.stderr);
    }
    Ok(())
}

pub(crate) fn print_legacy_logs_summary(
    runtime: &OrbitRuntime,
    run_id: &str,
    step_id: Option<&str>,
    json_output: bool,
) -> Result<(), OrbitError> {
    let run = runtime
        .show_job_run(run_id)
        .map_err(|_| OrbitError::JobRunNotFound(run_id.to_string()))?;
    let steps = filtered_steps(&run, step_id)?;

    if json_output {
        let values = steps
            .iter()
            .map(|step| legacy_step_to_json(step))
            .collect::<Vec<_>>();
        return crate::output::json::print_pretty(&Value::Array(values));
    }

    print_run_header(&run);
    println!();
    print_step_summary_table(&steps)
}

fn resolve_run(runtime: &OrbitRuntime, run_id: Option<&str>) -> Result<JobRun, OrbitError> {
    if let Some(run_id) = run_id {
        return runtime
            .show_job_run(run_id)
            .map_err(|_| OrbitError::JobRunNotFound(run_id.to_string()));
    }

    runtime
        .list_job_runs(JobRunListParams {
            limit: Some(1),
            ..Default::default()
        })?
        .into_iter()
        .next()
        .ok_or_else(|| OrbitError::JobRunNotFound("latest".to_string()))
}

fn find_run_step<'a>(run: &'a JobRun, step_id: &str) -> Result<&'a JobRunStep, OrbitError> {
    run.steps
        .iter()
        .find(|step| step.target_id == step_id || step.step_index.to_string() == step_id)
        .ok_or_else(|| {
            OrbitError::InvalidInput(format!(
                "step '{step_id}' does not match any step in run '{}'",
                run.run_id
            ))
        })
}

fn filtered_steps<'a>(
    run: &'a JobRun,
    step_id: Option<&str>,
) -> Result<Vec<&'a JobRunStep>, OrbitError> {
    match step_id {
        Some(step_id) => Ok(vec![find_run_step(run, step_id)?]),
        None => Ok(run.steps.iter().collect()),
    }
}

fn print_run_header(run: &JobRun) {
    use crate::output::color::{bold, dimmed, job_state_color};
    println!("{} {}", bold("Run ID:"), run.run_id);
    println!("{} {}", bold("Job ID:"), run.job_id);
    println!(
        "{} {}",
        bold("State:"),
        job_state_color(&run.state.to_string())
    );
    println!(
        "{} {}",
        bold("Started:"),
        dimmed(&format_timestamp(run.started_at))
    );
    println!(
        "{} {}",
        bold("Finished:"),
        dimmed(&format_timestamp(run.finished_at))
    );
    println!("{} {}", bold("Duration:"), format_duration(run.duration_ms));
}

fn print_step_summary_table(steps: &[&JobRunStep]) -> Result<(), OrbitError> {
    if steps.is_empty() {
        println!("No steps recorded.");
        return Ok(());
    }

    let mut table = crate::output::table::build_table(&[
        "#",
        "TARGET",
        "STATE",
        "DURATION",
        "ERROR CODE",
        "ERROR MESSAGE",
    ]);
    for step in steps {
        use comfy_table::Cell;
        table.add_row(vec![
            Cell::new(step.step_index),
            Cell::new(&step.target_id),
            crate::output::color::job_state_color_cell(&step.state.to_string()),
            Cell::new(format_duration(step.duration_ms)),
            Cell::new(step.error_code.as_deref().unwrap_or("-")),
            Cell::new(summarize_error_message(step.error_message.as_deref())),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn print_step_record(
    run: &JobRun,
    step: &JobRunStep,
    step_output: Option<Value>,
    json_output: bool,
) -> Result<(), OrbitError> {
    if json_output {
        return crate::output::json::print_pretty(&json!({
            "run_id": run.run_id,
            "job_id": run.job_id,
            "step": step_to_json(step),
            "step_output": step_output,
        }));
    }

    use crate::output::color::{bold, dimmed, job_state_color};
    println!("{} {}", bold("Run ID:"), run.run_id);
    println!("{} {}", bold("Job ID:"), run.job_id);
    println!("{} {}", bold("Target ID:"), step.target_id);
    println!("{} {}", bold("Target Type:"), step.target_type);
    println!(
        "{} {}",
        bold("State:"),
        job_state_color(&step.state.to_string())
    );
    println!(
        "{} {}",
        bold("Started:"),
        dimmed(&format_timestamp(step.started_at))
    );
    println!(
        "{} {}",
        bold("Finished:"),
        dimmed(&format_timestamp(step.finished_at))
    );
    println!(
        "{} {}",
        bold("Duration:"),
        format_duration(step.duration_ms)
    );
    println!(
        "{} {}",
        bold("Exit Code:"),
        step.exit_code
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Error Code:"),
        step.error_code.as_deref().unwrap_or("-")
    );
    println!(
        "{} {}",
        bold("Error Message:"),
        step.error_message.as_deref().unwrap_or("-")
    );
    if let Some(output) = step_output {
        println!("{}", bold("Step Output:"));
        println!(
            "{}",
            serde_json::to_string_pretty(&output)
                .map_err(|err| OrbitError::Store(err.to_string()))?
        );
    }
    Ok(())
}

fn step_to_json(step: &JobRunStep) -> Value {
    json!({
        "step_index": step.step_index,
        "target_id": step.target_id,
        "target_type": step.target_type.to_string(),
        "state": step.state.to_string(),
        "started_at": step.started_at.map(|t| t.to_rfc3339()),
        "finished_at": step.finished_at.map(|t| t.to_rfc3339()),
        "duration_ms": step.duration_ms,
        "exit_code": step.exit_code,
        "agent_response_json": step.agent_response_json,
        "error_code": step.error_code,
        "error_message": step.error_message,
    })
}

fn legacy_step_to_json(step: &JobRunStep) -> Value {
    json!({
        "step_index": step.step_index,
        "target_id": step.target_id,
        "target_type": step.target_type.to_string(),
        "state": step.state.to_string(),
        "started_at": step.started_at.map(|t| t.to_rfc3339()),
        "finished_at": step.finished_at.map(|t| t.to_rfc3339()),
        "duration_ms": step.duration_ms,
        "exit_code": step.exit_code,
        "error_code": step.error_code,
        "error_message": step.error_message,
    })
}

#[derive(Clone, Debug)]
struct RawLogRecord {
    step_id: Option<String>,
    provider: Option<String>,
    stdout_blob_ref: Option<String>,
    stderr_blob_ref: Option<String>,
    stdout: String,
    stderr: String,
}

fn collect_raw_log_records(
    runtime: &OrbitRuntime,
    run: &JobRun,
    step_id: Option<&str>,
) -> Result<Vec<RawLogRecord>, OrbitError> {
    let audit_path = runtime
        .data_root()
        .join("state")
        .join("audit")
        .join("v2_loop")
        .join(format!("{}.jsonl", run.run_id));
    if !audit_path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(&audit_path).map_err(|err| {
        OrbitError::Io(format!("read audit log '{}': {err}", audit_path.display()))
    })?;
    let mut events = HashMap::new();
    let mut ordered_ids = Vec::new();
    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let value: Value = serde_json::from_str(line).map_err(|err| {
            OrbitError::Store(format!(
                "invalid audit log '{}': {err}",
                audit_path.display()
            ))
        })?;
        let Some(event_id) = value.get("event_id").and_then(Value::as_str) else {
            continue;
        };
        ordered_ids.push(event_id.to_string());
        events.insert(event_id.to_string(), value);
    }

    let blob_store = BlobStore::new(
        runtime
            .data_root()
            .join("state")
            .join("audit")
            .join("blobs"),
    );
    let mut records = Vec::new();
    for event_id in ordered_ids {
        let Some(event) = events.get(&event_id) else {
            continue;
        };
        if event.get("body_kind").and_then(Value::as_str) != Some("cli_invocation_finished") {
            continue;
        }
        let event_step_id = enclosing_step_id(event, &events);
        if let Some(filter) = step_id
            && event_step_id.as_deref() != Some(filter)
        {
            continue;
        }
        let stdout_blob_ref = event
            .get("stdout_blob_ref")
            .and_then(Value::as_str)
            .map(str::to_string);
        let stderr_blob_ref = event
            .get("stderr_blob_ref")
            .and_then(Value::as_str)
            .map(str::to_string);
        let stdout = match stdout_blob_ref.as_deref() {
            Some(blob_ref) => read_blob_text(&blob_store, blob_ref)?,
            None => String::new(),
        };
        let stderr = match stderr_blob_ref.as_deref() {
            Some(blob_ref) => read_blob_text(&blob_store, blob_ref)?,
            None => String::new(),
        };
        records.push(RawLogRecord {
            step_id: event_step_id,
            provider: event
                .get("provider")
                .and_then(Value::as_str)
                .map(str::to_string),
            stdout_blob_ref,
            stderr_blob_ref,
            stdout,
            stderr,
        });
    }

    if let Some(step_id) = step_id {
        let step_exists_in_run = run
            .steps
            .iter()
            .any(|step| step.target_id == step_id || step.step_index.to_string() == step_id);
        if records.is_empty() && !step_exists_in_run {
            return Err(OrbitError::InvalidInput(format!(
                "step '{step_id}' does not match any step in run '{}'",
                run.run_id
            )));
        }
    }

    Ok(records)
}

fn enclosing_step_id(event: &Value, events: &HashMap<String, Value>) -> Option<String> {
    let mut parent_id = event
        .get("parent_event_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    while let Some(id) = parent_id {
        let parent = events.get(&id)?;
        if parent.get("body_kind").and_then(Value::as_str) == Some("step_started") {
            return parent
                .get("step_id")
                .and_then(Value::as_str)
                .map(str::to_string);
        }
        parent_id = parent
            .get("parent_event_id")
            .and_then(Value::as_str)
            .map(str::to_string);
    }
    None
}

fn read_blob_text(blob_store: &BlobStore, blob_ref: &str) -> Result<String, OrbitError> {
    if blob_ref.len() < 2 || blob_ref.starts_with("error:") {
        return Err(OrbitError::Store(format!(
            "invalid audit blob reference '{blob_ref}'"
        )));
    }
    let bytes = blob_store
        .read(blob_ref)
        .map_err(|err| OrbitError::Io(format!("read audit blob '{blob_ref}': {err}")))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn raw_log_record_to_json(record: &RawLogRecord) -> Value {
    json!({
        "step_id": record.step_id,
        "provider": record.provider,
        "stdout_blob_ref": record.stdout_blob_ref,
        "stderr_blob_ref": record.stderr_blob_ref,
        "stdout": record.stdout,
        "stderr": record.stderr,
    })
}

fn format_timestamp(value: Option<chrono::DateTime<chrono::Utc>>) -> String {
    value
        .map(|v| v.format("%Y-%m-%dT%H:%M:%SZ").to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_duration(value: Option<u64>) -> String {
    value
        .map(|duration| format!("{duration}ms"))
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::command::{Cli, Commands};

    use super::*;

    fn parse_run(args: &[&str]) -> RunCommand {
        let cli = Cli::parse_from(args);
        match cli.command {
            Commands::Run(command) => command,
            _ => panic!("expected run command"),
        }
    }

    #[test]
    fn parses_explicit_ship_defaults() {
        let command = parse_run(&["orbit", "run", "ship", "T1", "T2"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Ship(args) => {
                assert_eq!(args.task_ids, vec!["T1", "T2"]);
                assert_eq!(args.mode, ship::ShipMode::Pr);
                assert_eq!(args.base, "agent-main");
            }
            _ => panic!("expected ship"),
        }
    }

    #[test]
    fn parses_explicit_ship_mode_and_base() {
        let command = parse_run(&["orbit", "run", "ship", "-m", "local", "-b", "main", "T1"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Ship(args) => {
                assert_eq!(args.task_ids, vec!["T1"]);
                assert_eq!(args.mode, ship::ShipMode::Local);
                assert_eq!(args.base, "main");
            }
            _ => panic!("expected ship"),
        }
    }

    #[test]
    fn parses_ship_auto_as_top_level_subcommand() {
        let command = parse_run(&["orbit", "run", "ship-auto", "-m", "pr", "-b", "main"]);
        match command.command.expect("subcommand") {
            RunSubcommand::ShipAuto(args) => {
                assert_eq!(args.mode, ship::ShipMode::Pr);
                assert_eq!(args.base, "main");
            }
            _ => panic!("expected ship-auto"),
        }
    }

    #[test]
    fn parses_duel_plan_as_top_level_subcommand() {
        let command = parse_run(&["orbit", "run", "duel-plan", "T1", "-b", "main"]);
        match command.command.expect("subcommand") {
            RunSubcommand::DuelPlan(args) => {
                assert_eq!(args.task_id, "T1");
                assert_eq!(args.base, "main");
            }
            _ => panic!("expected duel-plan"),
        }
    }

    #[test]
    fn parses_run_job_unchanged() {
        let command = parse_run(&["orbit", "run", "job", "task_auto_pipeline", "--json"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Job(args) => {
                assert_eq!(args.job_id, "task_auto_pipeline");
                assert!(args.json);
            }
            _ => panic!("expected job"),
        }
    }

    #[test]
    fn parses_positional_job_fallback_unchanged() {
        let command = parse_run(&["orbit", "run", "task_auto_pipeline", "--json"]);
        assert!(command.command.is_none());
        assert_eq!(
            command.positional.job_id.as_deref(),
            Some("task_auto_pipeline")
        );
        assert!(command.positional.json);
    }

    #[test]
    fn parses_run_history_defaults() {
        let command = parse_run(&["orbit", "run", "history"]);
        match command.command.expect("subcommand") {
            RunSubcommand::History(args) => {
                assert_eq!(args.job_id, None);
                assert_eq!(args.limit, DEFAULT_HISTORY_LIMIT);
                assert!(!args.json);
            }
            _ => panic!("expected history"),
        }
    }

    #[test]
    fn parses_run_history_job_filter() {
        let command = parse_run(&["orbit", "run", "history", "-j", "task_auto_pipeline"]);
        match command.command.expect("subcommand") {
            RunSubcommand::History(args) => {
                assert_eq!(args.job_id.as_deref(), Some("task_auto_pipeline"));
                assert_eq!(args.limit, DEFAULT_HISTORY_LIMIT);
            }
            _ => panic!("expected history"),
        }
    }

    #[test]
    fn parses_run_show_latest() {
        let command = parse_run(&["orbit", "run", "show"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Show(args) => {
                assert_eq!(args.run_id, None);
                assert_eq!(args.step_id, None);
            }
            _ => panic!("expected show"),
        }
    }

    #[test]
    fn parses_run_show_run_id() {
        let command = parse_run(&["orbit", "run", "show", "jrun-1"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Show(args) => {
                assert_eq!(args.run_id.as_deref(), Some("jrun-1"));
                assert_eq!(args.step_id, None);
            }
            _ => panic!("expected show"),
        }
    }

    #[test]
    fn parses_run_show_step() {
        let command = parse_run(&["orbit", "run", "show", "jrun-1", "-s", "implement_one"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Show(args) => {
                assert_eq!(args.run_id.as_deref(), Some("jrun-1"));
                assert_eq!(args.step_id.as_deref(), Some("implement_one"));
            }
            _ => panic!("expected show"),
        }
    }

    #[test]
    fn parses_run_logs_latest() {
        let command = parse_run(&["orbit", "run", "logs"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Logs(args) => {
                assert_eq!(args.run_id, None);
                assert_eq!(args.step_id, None);
            }
            _ => panic!("expected logs"),
        }
    }

    #[test]
    fn parses_run_logs_run_id() {
        let command = parse_run(&["orbit", "run", "logs", "jrun-1"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Logs(args) => {
                assert_eq!(args.run_id.as_deref(), Some("jrun-1"));
                assert_eq!(args.step_id, None);
            }
            _ => panic!("expected logs"),
        }
    }

    #[test]
    fn parses_run_logs_step() {
        let command = parse_run(&["orbit", "run", "logs", "jrun-1", "-s", "implement_one"]);
        match command.command.expect("subcommand") {
            RunSubcommand::Logs(args) => {
                assert_eq!(args.run_id.as_deref(), Some("jrun-1"));
                assert_eq!(args.step_id.as_deref(), Some("implement_one"));
            }
            _ => panic!("expected logs"),
        }
    }

    #[test]
    fn rejects_removed_duel_history_forms() {
        assert!(Cli::try_parse_from(["orbit", "run", "duel", "list"]).is_err());
        assert!(Cli::try_parse_from(["orbit", "run", "duel", "show"]).is_err());
    }
}
