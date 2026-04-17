use std::process::Child;
use std::time::{Duration, Instant};

use orbit_types::OrbitError;
use wait_timeout::ChildExt;

#[cfg(unix)]
use super::cleanup::terminate_orphaned_process_group;
use super::cleanup::{kill_process_group, terminate_process_group, termination_signal};
#[cfg(unix)]
use super::signal::{SignalHandlerGuard, signal_message};
use super::tee::{spawn_stderr_drain, spawn_stdout_drain};

pub(crate) const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Output collected from a spawned process.
pub(crate) struct WaitResult {
    pub(crate) exit_success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: Vec<u8>,
    /// Stderr text; includes "process timed out" appended when timed out.
    pub(crate) stderr: Vec<u8>,
}

pub(crate) fn wait_with_optional_timeout(
    mut child: Child,
    timeout_ms: Option<u64>,
    debug: bool,
) -> Result<WaitResult, OrbitError> {
    // Drain stdout/stderr in background threads so the child never blocks on a
    // full pipe buffer (which would prevent it from exiting).
    //
    // In debug mode, stdout is tee'd to stderr so the user sees agent output
    // live while we still accumulate it for JSON parsing. Stderr is inherited
    // directly by the child process (set in process::spawn), so no thread is
    // needed for it.
    let stdout_thread = child
        .stdout
        .take()
        .map(|out| spawn_stdout_drain(out, debug));
    let stderr_thread = child.stderr.take().map(spawn_stderr_drain);

    #[cfg(unix)]
    let signal_guard = SignalHandlerGuard::install()?;

    let deadline = timeout_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
    let (timed_out, interrupted_signal, exit_success, exit_code) = loop {
        let wait_slice = deadline
            .map(|end| {
                end.saturating_duration_since(Instant::now())
                    .min(WAIT_POLL_INTERVAL)
            })
            .unwrap_or(WAIT_POLL_INTERVAL);

        if let Some(status) = child
            .wait_timeout(wait_slice)
            .map_err(|e| OrbitError::Execution(format!("wait timeout error: {e}")))?
        {
            #[cfg(unix)]
            if let Some(signal) = signal_guard.take_signal() {
                terminate_orphaned_process_group(child.id(), signal, WAIT_POLL_INTERVAL);
                break (false, Some(signal), false, Some(128 + signal));
            }

            // Child exited successfully within the timeout. Kill its process
            // group so any orphan subprocesses still holding the pipes open
            // are reaped before we join the reader threads below.
            kill_process_group(child.id());
            break (false, None, status.success(), status.code());
        }

        #[cfg(unix)]
        if let Some(signal) = signal_guard.take_signal() {
            terminate_process_group(&mut child, signal, WAIT_POLL_INTERVAL)?;
            break (false, Some(signal), false, Some(128 + signal));
        }

        if deadline.is_some_and(|end| Instant::now() >= end) {
            terminate_process_group(&mut child, termination_signal(), WAIT_POLL_INTERVAL)?;
            break (true, None, false, None);
        }
    };

    // Join reader threads. They complete quickly once the process group is
    // killed (all pipe write ends are closed -> EOF).
    let stdout = stdout_thread
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    let mut stderr = stderr_thread
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    if timed_out {
        if !stderr.is_empty() {
            stderr.push(b'\n');
        }
        stderr.extend_from_slice(b"process timed out");
    }
    #[cfg(unix)]
    if !timed_out {
        if let Some(signal) = interrupted_signal {
            if !stderr.is_empty() {
                stderr.push(b'\n');
            }
            stderr.extend_from_slice(signal_message(signal).as_bytes());
        }
    }

    #[cfg(not(unix))]
    let _ = interrupted_signal;

    Ok(WaitResult {
        exit_success,
        exit_code,
        stdout,
        stderr,
    })
}
