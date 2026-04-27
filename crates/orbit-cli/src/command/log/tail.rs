//! `orbit log tail` — column-formatted reader for the unified JSONL tracing
//! feed at `~/.orbit/state/logs/orbit.jsonl` (or wherever `--path` /
//! `ORBIT_LOG_PATH` points). Renders the v2-terminal-console mockup's four
//! columns: timestamp, source, code, message. Designed for human eyes by
//! default and pipeline-friendly when stdout is not a TTY (`--json` or
//! plain-text without ANSI escapes).

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use clap::{Args, ValueEnum};
use colored::{ColoredString, Colorize};
use orbit_core::{OrbitError, OrbitRuntime};
use serde_json::Value;

use crate::command::Execute;
use crate::parse::parse_since;

#[derive(Args)]
pub struct TailArgs {
    /// Number of recent lines to print before exiting (or before tailing in
    /// follow mode).
    #[arg(short = 'n', long, default_value_t = 50)]
    pub lines: usize,

    /// Tail the file as it grows. Stop on Ctrl-C.
    #[arg(short = 'f', long)]
    pub follow: bool,

    /// Filter by tracing target prefix (e.g. `--target orbit.policy`
    /// matches `orbit.policy.deny`).
    #[arg(long)]
    pub target: Option<String>,

    /// Filter by minimum log level. `error > warn > info > debug > trace`.
    #[arg(long)]
    pub level: Option<LevelFilter>,

    /// Filter by timestamp window (e.g. `5m`, `1h`, `30s`, RFC3339).
    #[arg(long)]
    pub since: Option<String>,

    /// Emit each event as one raw JSON line instead of the four-column view.
    #[arg(long)]
    pub json: bool,

    /// Override the JSONL path. Falls back to `$ORBIT_LOG_PATH`, then
    /// `$HOME/.orbit/state/logs/orbit.jsonl`. Provided primarily for tests.
    #[arg(long)]
    pub path: Option<PathBuf>,
}

impl Execute for TailArgs {
    fn execute(self, _runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let path = resolve_log_path(self.path.as_deref())?;
        let filters = build_filters(&self)?;
        let stdout = io::stdout();
        let use_color = stdout.is_terminal();
        let mut writer = stdout.lock();
        run_tail(&path, &self, &filters, use_color, &mut writer).map_err(io_to_orbit)
    }
}

#[derive(Clone, Copy, Debug, ValueEnum, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "lower")]
pub enum LevelFilter {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LevelFilter {
    fn rank(self) -> u8 {
        match self {
            LevelFilter::Trace => 0,
            LevelFilter::Debug => 1,
            LevelFilter::Info => 2,
            LevelFilter::Warn => 3,
            LevelFilter::Error => 4,
        }
    }

    fn from_event_level(level: &str) -> Option<LevelFilter> {
        match level.to_ascii_uppercase().as_str() {
            "TRACE" => Some(LevelFilter::Trace),
            "DEBUG" => Some(LevelFilter::Debug),
            "INFO" => Some(LevelFilter::Info),
            "WARN" => Some(LevelFilter::Warn),
            "ERROR" => Some(LevelFilter::Error),
            _ => None,
        }
    }
}

#[derive(Default)]
struct Filters {
    target_prefix: Option<String>,
    min_level: Option<LevelFilter>,
    since: Option<DateTime<Utc>>,
}

impl Filters {
    fn matches(&self, event: &Value) -> bool {
        let target = event.get("target").and_then(Value::as_str).unwrap_or("");
        if let Some(prefix) = &self.target_prefix
            && !target.starts_with(prefix)
        {
            return false;
        }
        if let Some(min) = self.min_level {
            let level = event.get("level").and_then(Value::as_str).unwrap_or("INFO");
            let event_level = LevelFilter::from_event_level(level).unwrap_or(LevelFilter::Info);
            if event_level.rank() < min.rank() {
                return false;
            }
        }
        if let Some(since) = self.since
            && let Some(ts) = event.get("timestamp").and_then(Value::as_str)
            && let Ok(parsed) = DateTime::parse_from_rfc3339(ts)
            && parsed.with_timezone(&Utc) < since
        {
            return false;
        }
        true
    }
}

fn build_filters(args: &TailArgs) -> Result<Filters, OrbitError> {
    let since = args.since.as_deref().map(parse_since).transpose()?;
    Ok(Filters {
        target_prefix: args.target.clone(),
        min_level: args.level,
        since,
    })
}

pub fn resolve_log_path(override_path: Option<&Path>) -> Result<PathBuf, OrbitError> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    if let Ok(env) = std::env::var("ORBIT_LOG_PATH")
        && !env.is_empty()
    {
        return Ok(PathBuf::from(env));
    }
    orbit_common::utility::logging::global_jsonl_log_path().map_err(|err| {
        OrbitError::InvalidInput(format!("cannot resolve global JSONL log path: {err}"))
    })
}

fn run_tail<W: Write>(
    path: &Path,
    args: &TailArgs,
    filters: &Filters,
    use_color: bool,
    writer: &mut W,
) -> io::Result<()> {
    if !path.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("orbit log file not found: {}", path.display()),
        ));
    }

    let initial_offset = print_initial_window(path, args, filters, use_color, writer)?;
    if !args.follow {
        return Ok(());
    }

    follow_file(path, initial_offset, filters, args.json, use_color, writer)
}

fn print_initial_window<W: Write>(
    path: &Path,
    args: &TailArgs,
    filters: &Filters,
    use_color: bool,
    writer: &mut W,
) -> io::Result<u64> {
    let file = File::open(path)?;
    let total_bytes = file.metadata()?.len();
    let mut reader = BufReader::new(file);
    let mut buf = String::new();
    let mut all = Vec::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            break;
        }
        all.push(buf.trim_end_matches('\n').to_string());
    }

    let kept: Vec<&String> = all
        .iter()
        .filter(|line| match serde_json::from_str::<Value>(line) {
            Ok(value) => filters.matches(&value),
            Err(_) => false,
        })
        .collect();

    let start = kept.len().saturating_sub(args.lines);
    for line in &kept[start..] {
        emit_line(line, args.json, use_color, writer)?;
    }
    Ok(total_bytes)
}

fn follow_file<W: Write>(
    path: &Path,
    initial_offset: u64,
    filters: &Filters,
    json: bool,
    use_color: bool,
    writer: &mut W,
) -> io::Result<()> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(initial_offset))?;
    let mut reader = BufReader::new(file);
    let mut leftover = String::new();

    loop {
        let mut buf = String::new();
        let n = reader.read_line(&mut buf)?;
        if n == 0 {
            thread::sleep(Duration::from_millis(50));
            continue;
        }
        if !buf.ends_with('\n') {
            // Partial line: stash and try again next iteration.
            leftover.push_str(&buf);
            continue;
        }
        let mut full_line = String::new();
        if !leftover.is_empty() {
            full_line.push_str(&leftover);
            leftover.clear();
        }
        full_line.push_str(buf.trim_end_matches('\n'));
        if let Ok(value) = serde_json::from_str::<Value>(&full_line)
            && filters.matches(&value)
        {
            emit_line(&full_line, json, use_color, writer)?;
        }
    }
}

fn emit_line<W: Write>(raw: &str, json: bool, use_color: bool, writer: &mut W) -> io::Result<()> {
    if json {
        writeln!(writer, "{raw}")?;
        return Ok(());
    }
    let value = match serde_json::from_str::<Value>(raw) {
        Ok(v) => v,
        Err(_) => {
            // Skip malformed lines silently: the producer warns about cross-process
            // interleaves, and reader robustness is part of the JSONL contract.
            return Ok(());
        }
    };
    let formatted = format_event_line(&value, use_color);
    writeln!(writer, "{formatted}")
}

pub(crate) fn format_event_line(event: &Value, use_color: bool) -> String {
    let timestamp = event
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("--:--:--");
    let level = event.get("level").and_then(Value::as_str).unwrap_or("INFO");
    let target = event.get("target").and_then(Value::as_str).unwrap_or("-");
    let fields = event
        .get("fields")
        .cloned()
        .unwrap_or_else(|| Value::Object(Default::default()));

    let time_col = format_timestamp(timestamp);
    let source_col = format_source(target, &fields);
    let code_col = format_code(target, level, &fields);
    let message_col = format_message(target, &fields);

    if use_color {
        format!(
            "{time}  {source:14}  {code}  {message}",
            time = time_col.dimmed(),
            source = colorize_source(target, &source_col),
            code = colorize_code(target, level, &code_col),
            message = message_col,
        )
    } else {
        format!("{time_col}  {source_col:14}  {code_col:5}  {message_col}")
    }
}

fn format_timestamp(raw: &str) -> String {
    // Accept ISO-8601 (RFC3339); display as HH:MM:SS in local-ish UTC. If the
    // string doesn't parse, render its first 8 chars after stripping the date.
    if let Ok(parsed) = DateTime::parse_from_rfc3339(raw) {
        return parsed.with_timezone(&Utc).format("%H:%M:%S").to_string();
    }
    if let Some(idx) = raw.find('T') {
        let after_t = &raw[idx + 1..];
        return after_t.chars().take(8).collect();
    }
    raw.chars().take(8).collect()
}

fn format_source(target: &str, fields: &Value) -> String {
    // High-value targets get short, fixed labels for the source column.
    if let Some(label) = match target {
        "orbit.policy.deny" => Some("policy"),
        "orbit.friction.reported" => Some("friction"),
        t if t.starts_with("orbit.job.") => Some("job"),
        _ => None,
    } {
        return label.to_string();
    }

    // cli_runner subprocess events: prefer the `provider` field as the source
    // so the reader sees `claude-4.5` / `codex` / etc. directly.
    if target == "orbit_engine::activity_job::cli_runner"
        && let Some(provider) = fields.get("provider").and_then(Value::as_str)
    {
        return provider.to_string();
    }

    // Generic fallback: tail of the dotted target.
    target
        .rsplit_once('.')
        .map(|(_, tail)| tail.to_string())
        .unwrap_or_else(|| target.to_string())
}

fn format_code(target: &str, level: &str, fields: &Value) -> String {
    match target {
        "orbit.policy.deny" => "DENY".to_string(),
        "orbit.friction.reported" => "FRC".to_string(),
        "orbit.job.step_retry" => "RTRY".to_string(),
        "orbit.job.step_finished" => match fields.get("success").and_then(Value::as_bool) {
            Some(true) => "OK".to_string(),
            Some(false) => "ERR".to_string(),
            None => "INF".to_string(),
        },
        _ => match level {
            "ERROR" => "ERR".to_string(),
            "WARN" => "WRN".to_string(),
            "INFO" => "INF".to_string(),
            "DEBUG" => "DBG".to_string(),
            "TRACE" => "TRC".to_string(),
            other => other.chars().take(3).collect::<String>().to_uppercase(),
        },
    }
}

pub(crate) fn format_message(target: &str, fields: &Value) -> String {
    let getf = |k: &str| fields.get(k).and_then(Value::as_str).unwrap_or("");
    let getn = |k: &str| -> String {
        fields
            .get(k)
            .map(|v| match v {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };

    match target {
        "orbit.policy.deny" => {
            let tool = getf("tool");
            let path = getf("path");
            let profile = getf("profile");
            let rule = getf("matched_rule");
            let mut s = String::new();
            if !tool.is_empty() {
                s.push_str(&format!("tool={tool}"));
            }
            if !path.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&format!("path={path}"));
            }
            if !profile.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&format!("profile={profile}"));
            }
            if !rule.is_empty() {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(&format!("rule={rule}"));
            }
            s
        }
        "orbit.friction.reported" => {
            let task_id = getf("task_id");
            let agent = getf("agent");
            let model = getf("model");
            let summary = getf("summary");
            let mut s = format!("friction reported on {task_id}");
            if !agent.is_empty() || !model.is_empty() {
                s.push_str(&format!(" by {agent}/{model}"));
            }
            if !summary.is_empty() {
                s.push_str(&format!(": {summary}"));
            }
            s
        }
        "orbit.job.step_started" => {
            format!(
                "step {} started [run={}]",
                getf("step_id"),
                getf("job_run_id"),
            )
        }
        "orbit.job.step_finished" => {
            let step = getf("step_id");
            let outcome = getf("outcome");
            let success = fields.get("success").and_then(Value::as_bool);
            match success {
                Some(true) => format!("step {step} finished ok ({outcome})"),
                Some(false) => format!("step {step} finished {outcome}"),
                None => format!("step {step} finished {outcome}"),
            }
        }
        "orbit.job.step_retry" => format!(
            "step {} retry attempt={} backoff_ms={}",
            getf("step_id"),
            getn("attempt"),
            getn("next_backoff_ms"),
        ),
        "orbit.job.step_skipped" => {
            format!("step {} skipped: {}", getf("step_id"), getf("reason"))
        }
        "orbit.job.step_denied" => {
            format!("step {} denied: {}", getf("step_id"), getf("reason"))
        }
        "orbit.job.fanout" => format!(
            "fanout phase={} step={} workers={} collected={} failed={}",
            getf("phase"),
            getf("step_id"),
            getn("worker_count"),
            getn("collected"),
            getn("failed"),
        ),
        "orbit.job.worker_state" => format!(
            "worker[{}] state={} step={}",
            getn("worker_index"),
            getf("state"),
            getf("step_id"),
        ),
        "orbit.job.loop_iteration" => format!(
            "loop {} phase={} step={}",
            getn("iteration"),
            getf("phase"),
            getf("step_id"),
        ),
        "orbit.job.loop_did_not_converge" => format!(
            "loop step={} did not converge after {} iterations",
            getf("step_id"),
            getn("max_iterations"),
        ),
        "orbit_engine::activity_job::cli_runner" => {
            let stream = getf("stream");
            let line = getf("line");
            if !stream.is_empty() {
                format!("[{stream}] {line}")
            } else {
                line.to_string()
            }
        }
        _ => {
            // Generic fallback: render fields as `key=value` space-separated,
            // omitting `message` (already handled above for known targets) and
            // `target` (already in the source column).
            let mut parts: Vec<String> = Vec::new();
            if let Value::Object(map) = fields {
                if let Some(message) = map.get("message").and_then(Value::as_str) {
                    parts.push(message.to_string());
                }
                for (k, v) in map {
                    if k == "message" {
                        continue;
                    }
                    let value_str = match v {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    parts.push(format!("{k}={value_str}"));
                }
            }
            parts.join(" ")
        }
    }
}

fn colorize_source(target: &str, label: &str) -> ColoredString {
    match target {
        "orbit.policy.deny" => label.red().bold(),
        "orbit.friction.reported" => label.yellow(),
        t if t.starts_with("orbit.job.") => label.cyan(),
        "orbit_engine::activity_job::cli_runner" => label.magenta(),
        _ => label.normal(),
    }
}

fn colorize_code(target: &str, level: &str, code: &str) -> ColoredString {
    if target == "orbit.policy.deny" {
        return code.red().bold();
    }
    match level {
        "ERROR" => code.red().bold(),
        "WARN" => code.yellow(),
        "INFO" => code.green(),
        "DEBUG" => code.blue(),
        _ => code.normal(),
    }
}

fn io_to_orbit(err: io::Error) -> OrbitError {
    OrbitError::InvalidInput(err.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use serde_json::json;
    use tempfile::tempdir;

    use super::*;

    fn fixture_lines() -> Vec<String> {
        vec![
            json!({
                "timestamp": "2026-04-27T01:00:01.123456789Z",
                "level": "INFO",
                "target": "orbit.job.step_started",
                "fields": {
                    "job_run_id": "run-1",
                    "task_id": "T123",
                    "step_id": "build",
                    "message": "step started"
                }
            })
            .to_string(),
            json!({
                "timestamp": "2026-04-27T01:00:02.000000000Z",
                "level": "INFO",
                "target": "orbit.job.step_finished",
                "fields": {
                    "job_run_id": "run-1",
                    "task_id": "T123",
                    "step_id": "build",
                    "outcome": "success",
                    "success": true,
                    "message": "step finished"
                }
            })
            .to_string(),
            json!({
                "timestamp": "2026-04-27T01:00:03.000000000Z",
                "level": "WARN",
                "target": "orbit.policy.deny",
                "fields": {
                    "tool": "fs.write",
                    "path": "/etc/passwd",
                    "profile": "writer",
                    "matched_rule": "/etc/**",
                    "message": "policy deny"
                }
            })
            .to_string(),
            json!({
                "timestamp": "2026-04-27T01:00:04.000000000Z",
                "level": "WARN",
                "target": "orbit.friction.reported",
                "fields": {
                    "task_id": "ORB-1011",
                    "agent": "codex",
                    "model": "gpt-5.5",
                    "summary": "tool docs missing",
                    "message": "friction reported"
                }
            })
            .to_string(),
            json!({
                "timestamp": "2026-04-27T01:00:05.000000000Z",
                "level": "INFO",
                "target": "orbit_engine::activity_job::cli_runner",
                "fields": {
                    "provider": "codex",
                    "stream": "stdout",
                    "job_run_id": "jrun-1",
                    "task_id": "T123",
                    "line": "hello world",
                    "message": "subprocess line"
                }
            })
            .to_string(),
        ]
    }

    fn write_fixture(path: &Path, lines: &[String]) {
        let mut content = String::new();
        for line in lines {
            content.push_str(line);
            content.push('\n');
        }
        std::fs::write(path, content).expect("write fixture");
    }

    fn capture(path: &Path, args: TailArgs) -> String {
        let filters = build_filters(&args).expect("build filters");
        let mut buf: Vec<u8> = Vec::new();
        run_tail(path, &args, &filters, false, &mut buf).expect("tail run");
        String::from_utf8(buf).expect("utf8")
    }

    fn make_args(path: PathBuf) -> TailArgs {
        TailArgs {
            lines: 50,
            follow: false,
            target: None,
            level: None,
            since: None,
            json: false,
            path: Some(path),
        }
    }

    #[test]
    fn default_tail_prints_last_n_formatted_columns_and_exits() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let output = capture(&path, make_args(path.clone()));
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines[0].contains("01:00:01"));
        assert!(lines[0].contains("job"));
        assert!(lines[0].contains("INF"));
        assert!(lines[0].contains("step build started"));
        assert!(lines[2].contains("DENY"));
        assert!(lines[2].contains("policy"));
        assert!(lines[2].contains("path=/etc/passwd"));
        assert!(lines[3].contains("FRC"));
        assert!(lines[3].contains("friction reported on ORB-1011"));
        assert!(lines[4].contains("codex"));
        assert!(lines[4].contains("[stdout] hello world"));
    }

    #[test]
    fn target_prefix_filter_matches_only_dotted_prefix() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let mut args = make_args(path.clone());
        args.target = Some("orbit.policy".to_string());
        let output = capture(&path, args);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("DENY"));
    }

    #[test]
    fn level_filter_drops_below_threshold() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let mut args = make_args(path.clone());
        args.level = Some(LevelFilter::Warn);
        let output = capture(&path, args);
        let lines: Vec<&str> = output.lines().collect();
        // INFO step_started + step_finished + cli_runner are dropped; WARN
        // policy.deny + friction.reported remain.
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("DENY"));
        assert!(lines[1].contains("FRC"));
    }

    #[test]
    fn since_filter_drops_older_events() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let mut args = make_args(path.clone());
        // Make `since` newer than the fixture's timestamps so only events
        // strictly after that cutoff would survive — but the fixture sits at
        // 2026-04-27T01:00:0X which is in the past relative to now-anchored
        // durations. Use a tiny window pinned to the future to assert the
        // filter actually drops.
        args.since = Some("0s".to_string());
        let output = capture(&path, args);
        // All fixture events have timestamps before "now-0s"; they should all
        // be dropped.
        assert_eq!(output.lines().count(), 0);
    }

    #[test]
    fn n_flag_limits_history() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let mut args = make_args(path.clone());
        args.lines = 2;
        let output = capture(&path, args);
        assert_eq!(output.lines().count(), 2);
        // Should be the last two: friction.reported + cli_runner.
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].contains("FRC"));
        assert!(lines[1].contains("[stdout] hello world"));
    }

    #[test]
    fn json_flag_emits_raw_lines_unchanged() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let mut args = make_args(path.clone());
        args.json = true;
        let output = capture(&path, args);
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 5);
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(*line, fixture_lines()[i]);
        }
    }

    #[test]
    fn non_tty_output_contains_no_ansi_escapes() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let output = capture(&path, make_args(path.clone()));
        assert!(
            !output.as_bytes().contains(&0x1b),
            "non-tty output leaked ANSI escape: {output}"
        );
    }

    #[test]
    fn follow_mode_emits_appended_line_within_window() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let path_clone = path.clone();
        let (tx, rx) = mpsc::channel::<String>();
        let handle = thread::spawn(move || {
            let mut buf = TeeWriter::new(tx);
            let args = TailArgs {
                lines: 0,
                follow: true,
                target: None,
                level: None,
                since: None,
                json: false,
                path: Some(path_clone.clone()),
            };
            let filters = build_filters(&args).expect("filters");
            // Tail should never return because of follow mode — we let the
            // join handle leak (test process exits when done).
            let _ = run_tail(&path_clone, &args, &filters, false, &mut buf);
        });

        // Give the follower a moment to seek to EOF and start polling.
        thread::sleep(Duration::from_millis(75));

        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append fixture");
        let appended = json!({
            "timestamp": "2026-04-27T01:00:06.000000000Z",
            "level": "INFO",
            "target": "orbit.job.step_started",
            "fields": {
                "job_run_id": "run-2",
                "step_id": "post-fixture",
                "message": "step started"
            }
        })
        .to_string();
        writeln!(file, "{appended}").expect("write appended");
        file.flush().ok();

        let deadline = Instant::now() + Duration::from_millis(500);
        let mut found = false;
        while Instant::now() < deadline {
            if let Ok(line) = rx.recv_timeout(Duration::from_millis(50))
                && line.contains("post-fixture")
            {
                found = true;
                break;
            }
        }

        // The follower thread is intentionally not joined; the test process
        // exits once the assertion completes.
        drop(handle);
        assert!(found, "follow mode did not surface appended line");
    }

    #[test]
    fn follow_mode_with_json_flag_emits_appended_line_as_raw_jsonl() {
        // Regression for review thread P2: follow mode must honor `--json` for
        // appended lines, not just for the initial window.
        let dir = tempdir().unwrap();
        let path = dir.path().join("orbit.jsonl");
        write_fixture(&path, &fixture_lines());

        let path_clone = path.clone();
        let (tx, rx) = mpsc::channel::<String>();
        let handle = thread::spawn(move || {
            let mut buf = TeeWriter::new(tx);
            let args = TailArgs {
                lines: 0,
                follow: true,
                target: None,
                level: None,
                since: None,
                json: true,
                path: Some(path_clone.clone()),
            };
            let filters = build_filters(&args).expect("filters");
            let _ = run_tail(&path_clone, &args, &filters, false, &mut buf);
        });

        thread::sleep(Duration::from_millis(75));

        let mut file = OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("append fixture");
        let appended_raw = json!({
            "timestamp": "2026-04-27T01:00:07.000000000Z",
            "level": "INFO",
            "target": "orbit.job.step_started",
            "fields": {
                "job_run_id": "run-3",
                "step_id": "json-followed",
                "message": "step started"
            }
        })
        .to_string();
        writeln!(file, "{appended_raw}").expect("write appended");
        file.flush().ok();

        let deadline = Instant::now() + Duration::from_millis(500);
        let mut got_raw = false;
        while Instant::now() < deadline {
            if let Ok(chunk) = rx.recv_timeout(Duration::from_millis(50)) {
                // Followed JSON output is the raw JSONL line — i.e. the same
                // string we appended, optionally followed by a newline. The
                // formatted four-column view would render `step json-followed
                // started [run=run-3]` instead, so asserting the literal raw
                // body is sufficient.
                if chunk.trim_end().ends_with(&appended_raw) {
                    got_raw = true;
                    break;
                }
            }
        }

        drop(handle);
        assert!(
            got_raw,
            "follow mode with --json did not surface appended line as raw JSONL",
        );
    }

    #[test]
    fn format_message_renders_each_high_value_target() {
        let policy = format_message(
            "orbit.policy.deny",
            &json!({
                "tool": "fs.write",
                "path": "/etc/passwd",
                "profile": "writer",
                "matched_rule": "/etc/**"
            }),
        );
        assert_eq!(
            policy,
            "tool=fs.write path=/etc/passwd profile=writer rule=/etc/**"
        );

        let friction = format_message(
            "orbit.friction.reported",
            &json!({
                "task_id": "ORB-1011",
                "agent": "codex",
                "model": "gpt-5.5",
                "summary": "missing"
            }),
        );
        assert!(friction.starts_with("friction reported on ORB-1011"));
        assert!(friction.contains("by codex/gpt-5.5"));
        assert!(friction.ends_with(": missing"));

        let started = format_message(
            "orbit.job.step_started",
            &json!({"job_run_id": "r", "step_id": "s"}),
        );
        assert_eq!(started, "step s started [run=r]");

        let finished_ok = format_message(
            "orbit.job.step_finished",
            &json!({"step_id": "s", "outcome": "success", "success": true}),
        );
        assert_eq!(finished_ok, "step s finished ok (success)");

        let runner = format_message(
            "orbit_engine::activity_job::cli_runner",
            &json!({
                "provider": "codex",
                "stream": "stderr",
                "line": "boom"
            }),
        );
        assert_eq!(runner, "[stderr] boom");
    }

    struct TeeWriter {
        tx: mpsc::Sender<String>,
    }

    impl TeeWriter {
        fn new(tx: mpsc::Sender<String>) -> Self {
            Self { tx }
        }
    }

    impl Write for TeeWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            if let Ok(text) = std::str::from_utf8(buf) {
                let _ = self.tx.send(text.to_string());
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
}
