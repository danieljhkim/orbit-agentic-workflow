use std::path::Path;
use std::thread;

use orbit_common::groundhog::FailureReport;
use orbit_exec::{EnvironmentMode, ExecRequest, NoSandbox, StdinMode, run_process};
use regex::Regex;
use serde::{Deserialize, Serialize};

pub use orbit_common::types::TaskPlanSuccessCriterion as Criterion;

pub const DEFAULT_OUTPUT_CAP_BYTES: usize = 4 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierConfig {
    pub output_cap_bytes: usize,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        Self {
            output_cap_bytes: DEFAULT_OUTPUT_CAP_BYTES,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionOutcome {
    Passed,
    Failed,
    SkippedSemantic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionRun {
    pub criterion: Criterion,
    pub outcome: CriterionOutcome,
    pub detail: String,
    pub exit_code: Option<i32>,
    pub captured_output: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierResult {
    Passed {
        runs: Vec<CriterionRun>,
    },
    Failed {
        report: FailureReport,
        runs: Vec<CriterionRun>,
    },
}

impl VerifierResult {
    pub fn runs(&self) -> &[CriterionRun] {
        match self {
            Self::Passed { runs } | Self::Failed { runs, .. } => runs,
        }
    }
}

pub fn verify_checkpoint(criteria: &[Criterion], workspace: &Path) -> VerifierResult {
    verify_checkpoint_with_config(criteria, workspace, VerifierConfig::default())
}

pub fn verify_checkpoint_with_config(
    criteria: &[Criterion],
    workspace: &Path,
    config: VerifierConfig,
) -> VerifierResult {
    let workspace = workspace.to_path_buf();
    let mut handles = Vec::with_capacity(criteria.len());

    for (index, criterion) in criteria.iter().cloned().enumerate() {
        let workspace = workspace.clone();
        let panic_criterion = criterion.clone();
        let output_cap_bytes = config.output_cap_bytes;
        let handle =
            thread::spawn(move || evaluate_criterion(criterion, &workspace, output_cap_bytes));
        handles.push((index, panic_criterion, handle));
    }

    // Preserve author-specified criterion order even though evaluation runs in parallel.
    let mut indexed_runs = Vec::with_capacity(handles.len());
    for (index, criterion, handle) in handles {
        let run = match handle.join() {
            Ok(run) => run,
            Err(_) => {
                let detail = format!(
                    "{} panicked while evaluating the criterion",
                    criterion_label(&criterion)
                );
                failed_run(criterion, detail, String::new(), None)
            }
        };
        indexed_runs.push((index, run));
    }
    indexed_runs.sort_by_key(|(index, _)| *index);
    let runs: Vec<CriterionRun> = indexed_runs.into_iter().map(|(_, run)| run).collect();

    if let Some(run) = runs
        .iter()
        .find(|run| run.outcome == CriterionOutcome::Failed)
    {
        VerifierResult::Failed {
            report: failure_report_from_run(run, config.output_cap_bytes),
            runs,
        }
    } else {
        VerifierResult::Passed { runs }
    }
}

fn evaluate_criterion(
    criterion: Criterion,
    workspace: &Path,
    output_cap_bytes: usize,
) -> CriterionRun {
    match &criterion {
        Criterion::Command {
            command,
            expect_exit,
        } => evaluate_command(
            criterion.clone(),
            workspace,
            command,
            *expect_exit,
            output_cap_bytes,
        ),
        Criterion::FileExists { path } => evaluate_file_exists(criterion.clone(), workspace, path),
        Criterion::FileContains { path, pattern } => evaluate_file_contains(
            criterion.clone(),
            workspace,
            path,
            pattern,
            output_cap_bytes,
        ),
        Criterion::Semantic { statement } => CriterionRun {
            criterion: criterion.clone(),
            outcome: CriterionOutcome::SkippedSemantic,
            detail: format!("semantic criterion `{statement}` skipped for agent judgment"),
            exit_code: None,
            captured_output: String::new(),
        },
    }
}

fn evaluate_command(
    criterion: Criterion,
    workspace: &Path,
    command: &str,
    expect_exit: i32,
    output_cap_bytes: usize,
) -> CriterionRun {
    let request = ExecRequest {
        program: "sh".to_string(),
        args: vec!["-lc".to_string(), command.to_string()],
        current_dir: Some(workspace.to_string_lossy().into_owned()),
        timeout_ms: None,
        stdin_mode: StdinMode::Null,
        environment_mode: EnvironmentMode::Inherit,
        debug: false,
    };

    match run_process(&request, &NoSandbox) {
        Ok(result) => {
            let captured_output = truncate_output(
                &combine_command_output(&result.stdout, &result.stderr),
                output_cap_bytes,
            );
            if result.exit_code == Some(expect_exit) {
                CriterionRun {
                    criterion,
                    outcome: CriterionOutcome::Passed,
                    detail: format!(
                        "command criterion `{command}` exited with {} as expected",
                        result.exit_code.unwrap_or_default()
                    ),
                    exit_code: result.exit_code,
                    captured_output,
                }
            } else {
                failed_run(
                    criterion,
                    format!(
                        "command criterion `{command}` exited with {:?}; expected {expect_exit}",
                        result.exit_code
                    ),
                    captured_output,
                    result.exit_code,
                )
            }
        }
        Err(error) => failed_run(
            criterion,
            format!("command criterion `{command}` failed to execute: {error}"),
            String::new(),
            None,
        ),
    }
}

fn evaluate_file_exists(criterion: Criterion, workspace: &Path, path: &str) -> CriterionRun {
    let resolved = workspace.join(path);
    if resolved.exists() {
        CriterionRun {
            criterion,
            outcome: CriterionOutcome::Passed,
            detail: format!(
                "file_exists criterion `{path}` found `{}`",
                resolved.display()
            ),
            exit_code: None,
            captured_output: String::new(),
        }
    } else {
        failed_run(
            criterion,
            format!(
                "file_exists criterion `{path}` did not find `{}`",
                resolved.display()
            ),
            String::new(),
            None,
        )
    }
}

fn evaluate_file_contains(
    criterion: Criterion,
    workspace: &Path,
    path: &str,
    pattern: &str,
    output_cap_bytes: usize,
) -> CriterionRun {
    let resolved = workspace.join(path);
    let contents = match std::fs::read_to_string(&resolved) {
        Ok(contents) => contents,
        Err(error) => {
            return failed_run(
                criterion,
                format!(
                    "file_contains criterion `{path}` could not read `{}`: {error}",
                    resolved.display()
                ),
                String::new(),
                None,
            );
        }
    };

    let regex = match Regex::new(pattern) {
        Ok(regex) => regex,
        Err(error) => {
            return failed_run(
                criterion,
                format!("file_contains criterion `{path}` has invalid regex `{pattern}`: {error}"),
                String::new(),
                None,
            );
        }
    };

    if regex.is_match(&contents) {
        CriterionRun {
            criterion,
            outcome: CriterionOutcome::Passed,
            detail: format!("file_contains criterion `{path}` matched regex `{pattern}`"),
            exit_code: None,
            captured_output: truncate_output(&contents, output_cap_bytes),
        }
    } else {
        failed_run(
            criterion,
            format!("file_contains criterion `{path}` did not match regex `{pattern}`"),
            truncate_output(&contents, output_cap_bytes),
            None,
        )
    }
}

fn failed_run(
    criterion: Criterion,
    detail: String,
    captured_output: String,
    exit_code: Option<i32>,
) -> CriterionRun {
    CriterionRun {
        criterion,
        outcome: CriterionOutcome::Failed,
        detail,
        exit_code,
        captured_output,
    }
}

fn failure_report_from_run(run: &CriterionRun, _output_cap_bytes: usize) -> FailureReport {
    let captured_output_note = if run.captured_output.is_empty() {
        String::new()
    } else {
        format!("\nCaptured output:\n{}", run.captured_output)
    };

    FailureReport {
        what_tried: format!("verified {}", criterion_label(&run.criterion)),
        what_happened: format!("{}{}", run.detail, captured_output_note),
        next_attempt_plan: next_attempt_plan(&run.criterion),
    }
}

fn next_attempt_plan(criterion: &Criterion) -> String {
    match criterion {
        Criterion::Command { .. } => {
            "Fix the failing command or its workspace preconditions, then retry the checkpoint."
                .to_string()
        }
        Criterion::FileExists { .. } => {
            "Create or restore the expected file before retrying the checkpoint.".to_string()
        }
        Criterion::FileContains { .. } => {
            "Update the file contents to satisfy the expected regex before retrying the checkpoint."
                .to_string()
        }
        Criterion::Semantic { .. } => {
            "Revisit the semantic criterion judgment before retrying the checkpoint.".to_string()
        }
    }
}

fn criterion_label(criterion: &Criterion) -> String {
    match criterion {
        Criterion::Command { command, .. } => format!("command criterion `{command}`"),
        Criterion::FileExists { path } => format!("file_exists criterion `{path}`"),
        Criterion::FileContains { path, pattern } => {
            format!("file_contains criterion `{path}` matching `{pattern}`")
        }
        Criterion::Semantic { statement } => format!("semantic criterion `{statement}`"),
    }
}

fn combine_command_output(stdout: &str, stderr: &str) -> String {
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!("stdout:\n{stdout}"),
        (true, false) => format!("stderr:\n{stderr}"),
        (false, false) => format!("stdout:\n{stdout}\n\nstderr:\n{stderr}"),
    }
}

fn truncate_output(text: &str, max_bytes: usize) -> String {
    if max_bytes == 0 || text.is_empty() {
        return String::new();
    }
    if text.len() <= max_bytes {
        return text.to_string();
    }

    const SUFFIX: &str = "...[truncated]";
    if max_bytes <= SUFFIX.len() {
        let mut end = max_bytes;
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        return text[..end].to_string();
    }

    let prefix_budget = max_bytes - SUFFIX.len();
    let mut safe_prefix_end = prefix_budget;
    while safe_prefix_end > 0 && !text.is_char_boundary(safe_prefix_end) {
        safe_prefix_end -= 1;
    }

    if safe_prefix_end == 0 {
        SUFFIX.to_string()
    } else {
        format!("{}{}", &text[..safe_prefix_end], SUFFIX)
    }
}
