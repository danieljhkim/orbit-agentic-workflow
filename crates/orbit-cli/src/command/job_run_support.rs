use chrono::{Duration, Utc};
use orbit_core::command::job_run::JobRunListParams;
use orbit_core::{JobRun, JobRunState, JobRunStep, OrbitError, OrbitRuntime};
use serde_json::{Value, json};

#[derive(Debug, Clone, Default)]
pub(crate) struct RunHistoryFilter {
    pub status: Option<JobRunState>,
    pub since: Option<String>,
    pub limit: Option<usize>,
}

pub(crate) fn load_filtered_job_runs(
    runtime: &OrbitRuntime,
    job_ids: &[&str],
    filter: &RunHistoryFilter,
) -> Result<Vec<JobRun>, OrbitError> {
    let since = filter
        .since
        .as_deref()
        .map(crate::parse::parse_duration_seconds)
        .transpose()?
        .map(|seconds| Utc::now() - Duration::seconds(seconds as i64));

    let mut runs = runtime.list_job_runs(JobRunListParams {
        job_id: None,
        state: filter.status,
        since,
        limit: None,
    })?;
    runs.retain(|run| job_ids.contains(&run.job_id.as_str()));
    runs.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.run_id.cmp(&left.run_id))
    });
    if let Some(limit) = filter.limit {
        runs.truncate(limit);
    }
    Ok(runs)
}

pub(crate) fn load_latest_job_run(
    runtime: &OrbitRuntime,
    job_ids: &[&str],
    label: &str,
) -> Result<JobRun, OrbitError> {
    load_filtered_job_runs(
        runtime,
        job_ids,
        &RunHistoryFilter {
            limit: Some(1),
            ..RunHistoryFilter::default()
        },
    )?
    .into_iter()
    .next()
    .ok_or_else(|| OrbitError::InvalidInput(format!("no {label} runs found")))
}

pub(crate) fn print_job_run_list(runs: &[JobRun]) {
    let mut table = crate::output::table::build_table(&[
        "RUN_ID",
        "JOB_ID",
        "ATTEMPT",
        "STATE",
        "STARTED_AT",
        "FINISHED_AT",
        "ERROR_CODE",
        "ERROR_MESSAGE",
    ]);
    for run in runs {
        use comfy_table::Cell;
        table.add_row(vec![
            Cell::new(&run.run_id),
            Cell::new(&run.job_id),
            Cell::new(run.attempt.to_string()),
            crate::output::color::job_state_color_cell(&run.state.to_string()),
            Cell::new(
                run.started_at
                    .map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(
                run.finished_at
                    .map(|value| value.format("%Y-%m-%dT%H:%M:%SZ").to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(
                run.steps
                    .last()
                    .and_then(|step| step.error_code.clone())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(summarize_error_message(
                run.steps
                    .last()
                    .and_then(|step| step.error_message.as_deref()),
            )),
        ]);
    }
    println!("{table}");
}

pub(crate) fn job_run_to_json(run: &JobRun) -> Value {
    let last = run.steps.last();
    json!({
        "run_id": run.run_id,
        "job_id": run.job_id,
        "attempt": run.attempt,
        "state": run.state.to_string(),
        "scheduled_at": run.scheduled_at.to_rfc3339(),
        "started_at": run.started_at.map(|value| value.to_rfc3339()),
        "finished_at": run.finished_at.map(|value| value.to_rfc3339()),
        "duration_ms": run.duration_ms,
        "exit_code": last.and_then(|step| step.exit_code),
        "agent_response_json": last.and_then(|step| step.agent_response_json.as_ref()),
        "error_code": last.and_then(|step| step.error_code.as_deref()),
        "error_message": last.and_then(|step| step.error_message.as_deref()),
        "knowledge_metrics": run.knowledge_metrics,
        "steps": run.steps.iter().map(job_run_step_to_json).collect::<Vec<_>>(),
        "created_at": run.created_at.to_rfc3339(),
    })
}

pub(crate) fn job_run_step_to_json(step: &JobRunStep) -> Value {
    json!({
        "step_index": step.step_index,
        "target_type": step.target_type.to_string(),
        "target_id": step.target_id,
        "state": step.state.to_string(),
        "started_at": step.started_at.map(|value| value.to_rfc3339()),
        "finished_at": step.finished_at.map(|value| value.to_rfc3339()),
        "duration_ms": step.duration_ms,
        "exit_code": step.exit_code,
        "agent_response_json": step.agent_response_json,
        "error_code": step.error_code,
        "error_message": step.error_message,
    })
}

pub(crate) fn summarize_error_message(raw: Option<&str>) -> String {
    let value = raw.unwrap_or("-").replace('\n', " ");
    if value.chars().count() <= 120 {
        return value;
    }
    let truncated = value.chars().take(120).collect::<String>();
    format!("{truncated}...")
}

pub(crate) fn print_job_run(run: &JobRun) {
    use crate::output::color::{bold, dimmed, job_state_color};
    println!("{} {}", bold("Run ID:"), run.run_id);
    println!("{} {}", bold("Job ID:"), run.job_id);
    println!("{} {}", bold("Attempt:"), run.attempt);
    println!(
        "{} {}",
        bold("State:"),
        job_state_color(&run.state.to_string())
    );
    println!(
        "{} {}",
        bold("Scheduled:"),
        dimmed(&run.scheduled_at.to_rfc3339())
    );
    println!(
        "{} {}",
        bold("Started:"),
        run.started_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Finished:"),
        run.finished_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Duration (ms):"),
        run.duration_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Created:"),
        dimmed(&run.created_at.to_rfc3339())
    );

    if run.steps.is_empty() {
        println!("\n{}", bold("Steps: (none)"));
        return;
    }

    println!("\n{}", bold("Steps:"));
    let mut table = crate::output::table::build_table(&[
        "STEP",
        "TARGET_ID",
        "STATE",
        "DURATION_MS",
        "EXIT_CODE",
        "ERROR_CODE",
        "ERROR_MESSAGE",
    ]);
    for step in &run.steps {
        use comfy_table::Cell;
        table.add_row(vec![
            Cell::new(step.step_index.to_string()),
            Cell::new(&step.target_id),
            crate::output::color::job_state_color_cell(&step.state.to_string()),
            Cell::new(
                step.duration_ms
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(
                step.exit_code
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            ),
            Cell::new(step.error_code.as_deref().unwrap_or("-")),
            Cell::new(summarize_error_message(step.error_message.as_deref())),
        ]);
    }
    println!("{table}");
    println!(
        "  {}",
        dimmed("Use --step <n> to inspect full details for a step.")
    );
}

pub(crate) fn print_step_detail(step: &JobRunStep) {
    use crate::output::color::{bold, dimmed, job_state_color};
    println!("{} {}", bold("Step Index:"), step.step_index);
    println!("{} {}", bold("Target Type:"), step.target_type);
    println!("{} {}", bold("Target ID:"), step.target_id);
    println!(
        "{} {}",
        bold("State:"),
        job_state_color(&step.state.to_string())
    );
    println!(
        "{} {}",
        bold("Started:"),
        step.started_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Finished:"),
        step.finished_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_else(|| "-".to_string())
    );
    println!(
        "{} {}",
        bold("Duration (ms):"),
        step.duration_ms
            .map(|value| value.to_string())
            .unwrap_or_else(|| "-".to_string())
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
    if let Some(response) = &step.agent_response_json {
        let rendered =
            serde_json::to_string_pretty(response).unwrap_or_else(|_| "<invalid-json>".to_string());
        println!("{}", bold("Agent Response:"));
        for line in rendered.lines() {
            println!("  {}", dimmed(line));
        }
    } else {
        println!("{} -", bold("Agent Response:"));
    }
}
