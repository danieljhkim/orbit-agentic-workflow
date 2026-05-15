//! Stable cross-process identity tokens for job-run owner verification.
//!
//! On Unix, the token is derived from `ps -o lstart=` with the child
//! environment forced to `TZ=UTC` / `LC_ALL=C` / `LANG=C` so the persisted
//! value does not depend on the caller's locale or timezone. Tokens written
//! by this helper carry a [`STABLE_TOKEN_PREFIX`] so readers can distinguish
//! them from legacy unversioned values.

/// Prefix on versioned identity tokens. Persisted tokens that start with this
/// prefix were written by the stable strategy and must match exactly.
pub const STABLE_TOKEN_PREFIX: &str = "ps-lstart-utc-v1:";

#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
fn lstart_raw(pid: u32, stable_env: bool) -> Option<String> {
    let mut cmd = Command::new("ps");
    cmd.args(["-o", "lstart=", "-p", &pid.to_string()]);
    if stable_env {
        cmd.env("TZ", "UTC").env("LC_ALL", "C").env("LANG", "C");
    }
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// Versioned, locale/timezone-stable process-start identity token. Writers and
/// readers must use this for new records so the value cannot diverge across
/// environments.
#[cfg(unix)]
pub fn process_start_identity_token(pid: u32) -> Option<String> {
    lstart_raw(pid, true).map(|raw| format!("{STABLE_TOKEN_PREFIX}{raw}"))
}

#[cfg(not(unix))]
pub fn process_start_identity_token(_pid: u32) -> Option<String> {
    None
}

/// Returns true when `persisted` is a legacy unversioned token whose value
/// matches either the caller-environment `ps -o lstart=` output or the
/// stable-environment one for this PID. Versioned tokens always return false
/// here so callers route them through [`process_start_identity_token`].
#[cfg(unix)]
pub fn legacy_lstart_matches(pid: u32, persisted: &str) -> bool {
    if persisted.starts_with(STABLE_TOKEN_PREFIX) {
        return false;
    }
    if let Some(stable_raw) = lstart_raw(pid, true)
        && stable_raw == persisted
    {
        return true;
    }
    if let Some(ambient) = lstart_raw(pid, false)
        && ambient == persisted
    {
        return true;
    }
    false
}

#[cfg(not(unix))]
pub fn legacy_lstart_matches(_pid: u32, _persisted: &str) -> bool {
    false
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn token_is_versioned_and_stable_for_self_pid() {
        let pid = std::process::id();
        let Some(first) = process_start_identity_token(pid) else {
            return;
        };
        assert!(
            first.starts_with(STABLE_TOKEN_PREFIX),
            "token must carry the versioned prefix: {first}"
        );
        let second = process_start_identity_token(pid).expect("second token");
        assert_eq!(first, second, "stable token must be deterministic");
    }

    #[test]
    fn legacy_match_rejects_versioned_input() {
        let pid = std::process::id();
        let Some(versioned) = process_start_identity_token(pid) else {
            return;
        };
        assert!(
            !legacy_lstart_matches(pid, &versioned),
            "versioned tokens must not be accepted via the legacy path"
        );
    }

    #[test]
    fn dead_pid_yields_no_token() {
        // PIDs near u32::MAX cannot exist on any supported platform.
        assert!(process_start_identity_token(u32::MAX - 1).is_none());
        assert!(!legacy_lstart_matches(u32::MAX - 1, "anything"));
    }
}
