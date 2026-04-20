//! Run-bundle helpers that share unix-specific primitives with `run.rs`.

#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
pub(super) fn process_start_time_token(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-o", "lstart=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

#[cfg(not(unix))]
pub(super) fn process_start_time_token(_pid: u32) -> Option<String> {
    None
}
