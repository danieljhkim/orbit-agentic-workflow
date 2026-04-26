//! Tracing subscriber setup.
//!
//! One canonical initializer for any Orbit binary. Libraries should emit
//! via `tracing::{info, warn, error, debug, trace}` and never touch the
//! subscriber.
//!
//! `init_default_subscriber` writes human-readable fmt output to stderr and,
//! when possible, also appends machine-readable JSON Lines to
//! `$HOME/.orbit/state/logs/orbit.jsonl`. The JSONL feed is global rather than
//! workspace-local because logging starts before CLI argument parsing and
//! runtime root resolution.
//!
//! JSONL retention is intentionally simple in v1: the file is append-only and
//! has no rotation. Multiple Orbit processes may append to the same file at
//! the same time; readers should tolerate malformed lines because writes
//! larger than `PIPE_BUF` can interleave across processes.
//!
//! Replacement guidance: the 13+ stray `eprintln!`/`println!` calls in
//! library crates (orbit-core, orbit-engine, orbit-store, orbit-knowledge)
//! should become `tracing::warn!` / `tracing::error!` with structured
//! fields. A workspace-level `deny(clippy::print_stderr, clippy::print_stdout)`
//! would enforce this but is left to follow-up.
//!
//! Redaction integration: today this module exposes [`redact_event_text`]
//! for manual pre-emission scrubbing. A proper `tracing::Layer` that applies
//! redaction to recorded fields is a TODO — the right shape is a
//! `tracing_subscriber::Layer` with a `Visit`-implementing formatter that
//! routes each value through [`super::redaction::redact_all`]. Landing that
//! layer is the follow-up after migrating call sites off `eprintln!`. Until
//! then, the JSONL file records fields as emitted by the call site; callers
//! that need scrubbing must pre-redact with [`redact_event_text`].

use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, registry::LookupSpan,
    util::SubscriberInitExt,
};

use super::redaction;

static FILE_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Install the default fmt + env-filter subscriber. Safe to call multiple
/// times — subsequent calls are no-ops (mirrors the current behaviour in
/// `orbit-cli/src/main.rs`).
///
/// `default_filter` is applied when `RUST_LOG` is unset (e.g. `"warn"`,
/// `"orbit=debug"`).
pub fn init_default_subscriber(default_filter: &str) {
    let filter = env_filter(default_filter);
    let stderr_layer = fmt::layer().with_writer(io::stderr);
    let log_layer = global_jsonl_log_path()
        .map_err(|err| err.to_string())
        .and_then(|path| jsonl_layer_at_path(&path).map_err(|err| err.to_string()));

    match log_layer {
        Ok((file_layer, guard)) => {
            if FILE_GUARD.set(guard).is_ok() {
                let _ = Registry::default()
                    .with(filter)
                    .with(stderr_layer)
                    .with(file_layer)
                    .try_init();
            } else {
                let _ = Registry::default()
                    .with(filter)
                    .with(stderr_layer)
                    .try_init();
                emit_log_init_warning("JSONL tracing worker guard was already initialized");
            }
        }
        Err(warning) => {
            let _ = Registry::default()
                .with(filter)
                .with(stderr_layer)
                .try_init();
            emit_log_init_warning(&warning);
        }
    }
}

/// Pre-emission scrubber for callers that need to sanitise a message before
/// handing it to `tracing::*!`. Applies env-value redaction plus the default
/// HTTP header/JSON patterns.
///
/// Prefer emitting structured fields and letting a future `RedactionLayer`
/// handle scrubbing uniformly; this helper exists so call sites can adopt
/// redaction today without blocking on the layer work.
pub fn redact_event_text(message: &str) -> String {
    redaction::redact_all(message)
}

fn env_filter(default_filter: &str) -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter))
}

fn global_jsonl_log_path() -> io::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "cannot resolve HOME/USERPROFILE for JSONL tracing log",
            )
        })?;

    Ok(PathBuf::from(home)
        .join(".orbit")
        .join("state")
        .join("logs")
        .join("orbit.jsonl"))
}

fn jsonl_layer_at_path<S>(
    path: &Path,
) -> io::Result<(impl Layer<S> + Send + Sync + 'static + use<S>, WorkerGuard)>
where
    S: tracing::Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "cannot create JSONL tracing log directory {}: {err}",
                    parent.display()
                ),
            )
        })?;
    }

    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| {
            io::Error::new(
                err.kind(),
                format!("cannot open JSONL tracing log {}: {err}", path.display()),
            )
        })?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    let layer = fmt::layer().json().with_ansi(false).with_writer(writer);

    Ok((layer, guard))
}

fn emit_log_init_warning(warning: &str) {
    tracing::warn!(
        target: "orbit_common::utility::logging",
        error = warning,
        "failed to initialize JSONL tracing log"
    );
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        io::Write,
        sync::{Arc, Mutex},
    };

    use regex::Regex;
    use serde_json::Value;
    use tempfile::tempdir;
    use tracing_subscriber::fmt::MakeWriter;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn jsonl_layer_honors_rust_log_filter() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _rust_log = EnvVarGuard::set("RUST_LOG", OsString::from("orbit_common=debug"));
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("orbit.jsonl");

        with_test_subscriber_at_path("trace", &log_path, io::sink, || {
            tracing::debug!(target: "orbit_common::filter_probe", accepted = true);
            tracing::trace!(target: "orbit_common::filter_probe", rejected = true);
        })
        .expect("subscriber should run");

        let values = read_jsonl_values(&log_path);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["level"], "DEBUG");
        assert_eq!(values[0]["fields"]["accepted"], true);
        assert!(values[0]["fields"].get("rejected").is_none());
    }

    #[test]
    fn jsonl_event_contains_required_shape_and_fields() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _rust_log = EnvVarGuard::remove("RUST_LOG");
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("orbit.jsonl");

        with_test_subscriber_at_path("info", &log_path, io::sink, || {
            tracing::info!(provider = "codex", stream = "stdout", line = "hi");
        })
        .expect("subscriber should run");

        let values = read_jsonl_values(&log_path);
        assert_eq!(values.len(), 1);
        let event = &values[0];
        let timestamp = event["timestamp"].as_str().expect("timestamp string");
        let timestamp_re =
            Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}").expect("valid regex");
        assert!(
            timestamp_re.is_match(timestamp),
            "timestamp should be ISO-like, got {timestamp}"
        );
        assert_eq!(event["level"], "INFO");
        assert!(event.get("target").is_some());
        assert_eq!(event["fields"]["provider"], "codex");
        assert_eq!(event["fields"]["stream"], "stdout");
        assert_eq!(event["fields"]["line"], "hi");
    }

    #[test]
    fn jsonl_event_preserves_cli_runner_structured_fields() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _rust_log = EnvVarGuard::remove("RUST_LOG");
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("orbit.jsonl");

        with_test_subscriber_at_path("info", &log_path, io::sink, || {
            tracing::info!(
                provider = "codex",
                stream = "stderr",
                job_run_id = "jrun-123",
                task_id = "T20260426-2343",
                line = "hello"
            );
        })
        .expect("subscriber should run");

        let values = read_jsonl_values(&log_path);
        assert_eq!(values.len(), 1);
        let fields = &values[0]["fields"];
        assert_eq!(fields["provider"], "codex");
        assert_eq!(fields["stream"], "stderr");
        assert_eq!(fields["job_run_id"], "jrun-123");
        assert_eq!(fields["task_id"], "T20260426-2343");
        assert_eq!(fields["line"], "hello");
    }

    #[test]
    fn jsonl_file_appends_to_existing_content() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _rust_log = EnvVarGuard::remove("RUST_LOG");
        let dir = tempdir().expect("tempdir");
        let log_path = dir.path().join("orbit.jsonl");
        fs::write(&log_path, "sentinel\n").expect("write sentinel");

        with_test_subscriber_at_path("info", &log_path, io::sink, || {
            tracing::info!(line = "after-sentinel");
        })
        .expect("subscriber should run");

        let content = fs::read_to_string(&log_path).expect("read log");
        let lines = content.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "sentinel");
        let appended: Value = serde_json::from_str(lines[1]).expect("appended line is json");
        assert_eq!(appended["fields"]["line"], "after-sentinel");
    }

    #[test]
    fn file_layer_failure_falls_back_to_stderr_layer() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _rust_log = EnvVarGuard::remove("RUST_LOG");
        let dir = tempdir().expect("tempdir");
        let blocked_parent = dir.path().join("not-a-directory");
        fs::write(&blocked_parent, "file, not dir").expect("write blocking file");
        let log_path = blocked_parent.join("orbit.jsonl");
        let stderr = BufferMakeWriter::default();
        let stderr_buffer = stderr.buffer();

        let warning = with_test_subscriber_allowing_file_failure("info", &log_path, stderr, || {
            tracing::info!(line = "stderr-still-works");
        })
        .expect("file layer should fail");

        assert!(warning.contains("cannot create JSONL tracing log directory"));
        let stderr_text = String::from_utf8(stderr_buffer.lock().expect("stderr lock").clone())
            .expect("stderr utf8");
        assert!(stderr_text.contains("failed to initialize JSONL tracing log"));
        assert!(stderr_text.contains("stderr-still-works"));
    }

    #[test]
    fn redact_event_text_still_scrubs_sensitive_text() {
        let _env = ENV_LOCK.lock().expect("lock env");
        let _secret = EnvVarGuard::set("ORBIT_TEST_TOKEN", OsString::from("super-secret-value"));

        let redacted = redact_event_text("token is super-secret-value");

        assert!(!redacted.contains("super-secret-value"));
        assert!(redacted.contains("[REDACTED_ENV]"));
    }

    fn with_test_subscriber_at_path<W>(
        default_filter: &str,
        log_path: &Path,
        stderr_writer: W,
        f: impl FnOnce(),
    ) -> io::Result<()>
    where
        W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
    {
        let filter = env_filter(default_filter);
        let stderr_layer = fmt::layer().with_writer(stderr_writer);
        let (file_layer, guard) = jsonl_layer_at_path(log_path)?;
        let subscriber = Registry::default()
            .with(filter)
            .with(stderr_layer)
            .with(file_layer);
        let dispatch = tracing::Dispatch::new(subscriber);
        tracing::dispatcher::with_default(&dispatch, f);
        drop(guard);
        Ok(())
    }

    fn with_test_subscriber_allowing_file_failure<W>(
        default_filter: &str,
        log_path: &Path,
        stderr_writer: W,
        f: impl FnOnce(),
    ) -> Option<String>
    where
        W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
    {
        let filter = env_filter(default_filter);
        let stderr_layer = fmt::layer().with_writer(stderr_writer);
        match jsonl_layer_at_path(log_path) {
            Ok((file_layer, guard)) => {
                let subscriber = Registry::default()
                    .with(filter)
                    .with(stderr_layer)
                    .with(file_layer);
                let dispatch = tracing::Dispatch::new(subscriber);
                tracing::dispatcher::with_default(&dispatch, f);
                drop(guard);
                None
            }
            Err(err) => {
                let warning = err.to_string();
                let subscriber = Registry::default().with(filter).with(stderr_layer);
                let dispatch = tracing::Dispatch::new(subscriber);
                tracing::dispatcher::with_default(&dispatch, || {
                    emit_log_init_warning(&warning);
                    f();
                });
                Some(warning)
            }
        }
    }

    fn read_jsonl_values(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .expect("read jsonl")
            .lines()
            .map(|line| serde_json::from_str(line).expect("valid json line"))
            .collect()
    }

    #[derive(Clone, Default)]
    struct BufferMakeWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl BufferMakeWriter {
        fn buffer(&self) -> Arc<Mutex<Vec<u8>>> {
            Arc::clone(&self.buffer)
        }
    }

    impl<'writer> MakeWriter<'writer> for BufferMakeWriter {
        type Writer = BufferWriter;

        fn make_writer(&'writer self) -> Self::Writer {
            BufferWriter {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

    struct BufferWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for BufferWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer
                .lock()
                .expect("buffer lock")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: OsString) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::set_var(key, value);
            }
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var_os(key);
            unsafe {
                std::env::remove_var(key);
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe {
                    std::env::set_var(self.key, value);
                },
                None => unsafe {
                    std::env::remove_var(self.key);
                },
            }
        }
    }
}
