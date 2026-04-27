//! macOS `sandbox-exec` primitive: SBPL compilation + sandboxed spawn.
//!
//! Translates a [`ResolvedFsProfile`] into a Sandbox Profile Language (SBPL)
//! payload and wraps a child process in `sandbox-exec -f <profile>`. This is
//! the OS-level enforcement seam for `backend: cli` activities.
//!
//! # Why not `--sandbox` flags on each agent CLI?
//!
//! Codex ships its own `--sandbox` flag, gemini has `-s`, claude has nothing
//! at the OS level. Building enforcement on three different CLI surfaces
//! produces three different audit stories and an asymmetric trust model.
//! Wrapping each invocation in `sandbox-exec` gives one declarative source
//! of truth — the activity's `FsProfile` — and one enforcement seam.
//!
//! # SBPL caveats
//!
//! Apple deprecated SBPL but the kernel still honors it (codex itself uses
//! it). v1 accepts that risk; the design doc records the choice. Negated
//! `!path` rules from `denyRead` / `denyModify` are emitted as explicit
//! `(deny file-read* (subpath ...))` / `(deny file-write* (subpath ...))`
//! clauses appended after the broad allows so they win in last-match-wins.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use orbit_common::types::{OrbitError, ResolvedFsProfile};
use tempfile::NamedTempFile;

/// Compile a [`ResolvedFsProfile`] into SBPL text suitable for
/// `sandbox-exec -f`.
///
/// The emitted profile:
/// - denies everything by default;
/// - allows broad reads (`file-read*`) — agent CLIs read from `/usr`,
///   `/System`, `/Library`, dyld caches, fonts, and similar locations that
///   are not realistic to enumerate;
/// - allows the syscall classes agent CLIs rely on (process, signal, mach,
///   ipc, sysctl, iokit) and unrestricted network — agents call out to
///   provider APIs;
/// - allows writes inside the resolved `modify` scope plus a small set of
///   well-known scratch areas (`/tmp`, `/private/tmp`,
///   `/private/var/folders`, `~/Library/Caches`) that tools and the
///   filesystem layer expect to write to;
/// - appends explicit `(deny ...)` clauses for any negated entry in
///   `read` / `modify` so global `denyRead` / `denyModify` rules win
///   under SBPL's last-match-wins evaluation.
///
/// Paths in `rules.modify` are emitted as-is. Callers must resolve
/// workspace-relative globs to absolute paths before invoking this
/// function — a relative `subpath` is meaningless to the kernel.
pub fn compile_macos_sandbox_profile(rules: &ResolvedFsProfile) -> Result<String, OrbitError> {
    let mut out = String::new();
    out.push_str("(version 1)\n");
    out.push_str("(deny default)\n");

    out.push_str("(allow file-read*)\n");
    out.push_str("(allow process*)\n");
    out.push_str("(allow signal)\n");
    out.push_str("(allow ipc-posix*)\n");
    out.push_str("(allow mach*)\n");
    out.push_str("(allow system-fsctl)\n");
    out.push_str("(allow system-socket)\n");
    out.push_str("(allow network*)\n");
    out.push_str("(allow sysctl*)\n");
    out.push_str("(allow iokit*)\n");

    out.push_str("(allow file-write* (subpath \"/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/var/folders\"))\n");
    out.push_str("(allow file-write* (subpath \"/dev\"))\n");
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}/Library/Caches\"))\n",
            sbpl_escape(&home)
        ));
        // The agent CLI inherits the sandbox into its `orbit mcp serve` child
        // (and any other `orbit ...` calls it makes). Those processes need
        // write access to the global Orbit data root so audit events, the
        // SQLite store, and run-state files can be persisted. Without this
        // the inherited child fails with `readonly database` and MCP tool
        // calls round-trip empty.
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}/.orbit\"))\n",
            sbpl_escape(&home)
        ));
    }

    for rule in &rules.modify {
        if let Some(deny_path) = rule.strip_prefix('!') {
            let path = subpath_root(deny_path);
            out.push_str(&format!(
                "(deny file-write* (subpath \"{}\"))\n",
                sbpl_escape(&path)
            ));
            continue;
        }
        let path = subpath_root(rule);
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            sbpl_escape(&path)
        ));
    }

    for rule in &rules.read {
        if let Some(deny_path) = rule.strip_prefix('!') {
            let path = subpath_root(deny_path);
            out.push_str(&format!(
                "(deny file-read* (subpath \"{}\"))\n",
                sbpl_escape(&path)
            ));
        }
    }

    Ok(out)
}

/// Spawn `program` under `sandbox-exec -f <profile>`. Returns the running
/// [`Child`] paired with a [`NamedTempFile`] holding the compiled profile;
/// the caller must keep the `NamedTempFile` alive until the child exits, or
/// the kernel may lose the profile mid-run.
///
/// `process_group(0)` is set on Unix so the supervision layer can reap the
/// whole tree (matching the `orbit-exec::process::spawn` contract).
pub fn spawn_under_macos_sandbox(
    profile_text: &str,
    program: &str,
    args: &[String],
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
) -> Result<(Child, NamedTempFile), OrbitError> {
    let mut profile_file = tempfile::Builder::new()
        .prefix("orbit-sandbox-")
        .suffix(".sb")
        .tempfile()
        .map_err(|err| {
            OrbitError::Execution(format!("failed to create sandbox profile tempfile: {err}"))
        })?;
    use std::io::Write;
    profile_file
        .write_all(profile_text.as_bytes())
        .map_err(|err| {
            OrbitError::Execution(format!("failed to write sandbox profile tempfile: {err}"))
        })?;
    profile_file
        .flush()
        .map_err(|err| OrbitError::Execution(format!("failed to flush sandbox profile: {err}")))?;

    let profile_path = profile_file.path().to_path_buf();

    let mut command = Command::new("sandbox-exec");
    command
        .arg("-f")
        .arg(&profile_path)
        .arg(program)
        .args(args)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let child = command.spawn().map_err(|err| {
        OrbitError::Execution(format!(
            "failed to spawn sandbox-exec wrapper around `{program}`: {err}"
        ))
    })?;
    Ok((child, profile_file))
}

/// Returns `true` if `sandbox-exec` is on `PATH`.
pub fn sandbox_exec_available() -> bool {
    sandbox_exec_available_in(&std::env::var_os("PATH").unwrap_or_default())
}

pub(crate) fn sandbox_exec_available_in(path_var: &std::ffi::OsStr) -> bool {
    for dir in std::env::split_paths(path_var) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join("sandbox-exec");
        if is_executable(&candidate) {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && (meta.permissions().mode() & 0o111) != 0,
        Err(_) => false,
    }
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn sbpl_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Strip glob suffixes from a rule so it can be used as a `subpath` root.
/// `subpath` matches a directory and everything beneath, so `**` wildcards
/// are redundant and `*` segments cannot be expressed in SBPL — we collapse
/// them to the longest non-glob prefix.
fn subpath_root(rule: &str) -> String {
    let trimmed = rule.trim_end_matches('/');
    let trimmed = trimmed.trim_end_matches("/**");
    if let Some(idx) = trimmed.find(|c: char| c == '*' || c == '?') {
        let prefix = &trimmed[..idx];
        let prefix = prefix.trim_end_matches('/');
        if prefix.is_empty() {
            "/".to_string()
        } else {
            prefix.to_string()
        }
    } else if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbit_common::types::ResolvedFsProfile;

    fn profile(name: &str, read: &[&str], modify: &[&str]) -> ResolvedFsProfile {
        ResolvedFsProfile {
            name: name.to_string(),
            read: read.iter().map(|s| s.to_string()).collect(),
            modify: modify.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn compile_emits_deny_default_and_broad_read_with_modify_subpath() {
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_macos_sandbox_profile(&resolved).expect("compile");
        assert!(text.contains("(deny default)"));
        assert!(text.contains("(allow file-read*)"));
        assert!(
            text.contains("(allow file-write* (subpath \"/Users/test/repo/src\"))"),
            "missing modify subpath clause: {text}"
        );
    }

    #[test]
    fn compile_grants_write_access_to_global_orbit_data_root() {
        // The agent CLI inherits the sandbox into `orbit mcp serve` and any
        // other `orbit ...` calls; those need to write to ~/.orbit (audit
        // events, SQLite stores, run state). Without this clause the
        // inherited child fails with `readonly database`.
        unsafe {
            std::env::set_var("HOME", "/Users/test");
        }
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_macos_sandbox_profile(&resolved).expect("compile");
        assert!(
            text.contains("(allow file-write* (subpath \"/Users/test/.orbit\"))"),
            "missing ~/.orbit write allow: {text}"
        );
    }

    #[test]
    fn compile_strips_glob_suffix_for_subpath_root() {
        let resolved = profile(
            "default",
            &["/Users/test/repo"],
            &["/Users/test/repo/src/**"],
        );
        let text = compile_macos_sandbox_profile(&resolved).expect("compile");
        assert!(
            text.contains("(allow file-write* (subpath \"/Users/test/repo/src\"))"),
            "expected glob-stripped subpath: {text}"
        );
        assert!(
            !text.contains("/src/**"),
            "subpath should not contain glob marker: {text}"
        );
    }

    #[test]
    fn compile_appends_explicit_deny_for_negated_modify_rule() {
        let mut resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo"]);
        resolved.modify.push("!/Users/test/repo/.env".to_string());
        let text = compile_macos_sandbox_profile(&resolved).expect("compile");
        assert!(
            text.contains("(deny file-write* (subpath \"/Users/test/repo/.env\"))"),
            "missing deny clause: {text}"
        );
        let allow_pos = text
            .find("(allow file-write* (subpath \"/Users/test/repo\"))")
            .expect("allow clause present");
        let deny_pos = text
            .find("(deny file-write* (subpath \"/Users/test/repo/.env\"))")
            .expect("deny clause present");
        assert!(
            allow_pos < deny_pos,
            "deny clause must come after allow for last-match-wins: {text}"
        );
    }

    #[test]
    fn sandbox_exec_available_in_finds_executable_on_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("sandbox-exec");
        std::fs::write(&bin, "#!/bin/sh\nexit 0\n").expect("write");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).expect("perms");
        }
        let path_var = std::ffi::OsString::from(dir.path().display().to_string());
        assert!(sandbox_exec_available_in(&path_var));
    }

    #[test]
    fn sandbox_exec_available_in_returns_false_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path_var = std::ffi::OsString::from(dir.path().display().to_string());
        assert!(!sandbox_exec_available_in(&path_var));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn compiled_profile_blocks_writes_outside_modify_scope() {
        use std::process::Command;

        // The compiled profile broadly allows writes under /tmp,
        // /private/tmp, /private/var/folders, and ~/Library/Caches so
        // agent CLIs can use scratch space. To exercise modify-scope
        // enforcement we need a parent that lives outside all of those.
        let home = std::env::var("HOME").expect("HOME set on macOS");
        let parent = std::path::PathBuf::from(home)
            .join(format!(".orbit-sandbox-test-{}", std::process::id()));
        std::fs::create_dir_all(&parent).expect("parent dir");
        let _cleanup = ScopeGuard(parent.clone());
        let dir = tempfile::Builder::new()
            .prefix("compile-")
            .tempdir_in(&parent)
            .expect("tempdir in parent");
        let allowed = dir.path().join("allowed");
        let blocked = dir.path().join("blocked");
        std::fs::create_dir_all(&allowed).expect("allowed dir");
        std::fs::create_dir_all(&blocked).expect("blocked dir");

        let resolved = ResolvedFsProfile {
            name: "default".to_string(),
            read: vec![dir.path().display().to_string()],
            modify: vec![allowed.display().to_string()],
        };
        let profile_text = compile_macos_sandbox_profile(&resolved).expect("compile sbpl");
        let mut profile_file = tempfile::Builder::new()
            .prefix("orbit-sandbox-test-")
            .suffix(".sb")
            .tempfile()
            .expect("tempfile");
        use std::io::Write;
        profile_file
            .write_all(profile_text.as_bytes())
            .expect("write profile");
        profile_file.flush().expect("flush");

        let allowed_target = allowed.join("ok");
        let allow_status = Command::new("sandbox-exec")
            .arg("-f")
            .arg(profile_file.path())
            .arg("/bin/sh")
            .arg("-c")
            .arg(format!("echo ok > {}", shell_escape(&allowed_target)))
            .status()
            .expect("run sandbox-exec");
        assert!(
            allow_status.success(),
            "expected write inside modify scope to succeed; status={allow_status:?}"
        );
        assert!(
            allowed_target.exists(),
            "allowed file was not written: {allowed_target:?}"
        );

        let blocked_target = blocked.join("nope");
        let deny_status = Command::new("sandbox-exec")
            .arg("-f")
            .arg(profile_file.path())
            .arg("/bin/sh")
            .arg("-c")
            .arg(format!("echo bad > {}", shell_escape(&blocked_target)))
            .status()
            .expect("run sandbox-exec");
        assert!(
            !deny_status.success(),
            "expected write outside modify scope to fail; status={deny_status:?}"
        );
        assert!(
            !blocked_target.exists(),
            "blocked file should not exist: {blocked_target:?}"
        );
    }

    #[cfg(target_os = "macos")]
    fn shell_escape(path: &Path) -> String {
        let s = path.display().to_string();
        format!("'{}'", s.replace('\'', "'\\''"))
    }

    #[cfg(target_os = "macos")]
    struct ScopeGuard(std::path::PathBuf);

    #[cfg(target_os = "macos")]
    impl Drop for ScopeGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
