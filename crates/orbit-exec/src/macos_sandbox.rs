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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
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
    let home = std::env::var_os("HOME");
    let codex_home = std::env::var_os("CODEX_HOME");
    compile_macos_sandbox_profile_with_env(rules, home.as_deref(), codex_home.as_deref())
}

fn compile_macos_sandbox_profile_with_env(
    rules: &ResolvedFsProfile,
    home: Option<&OsStr>,
    codex_home: Option<&OsStr>,
) -> Result<String, OrbitError> {
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
    // Codex's own seatbelt profile allows these provenance-related MAC
    // syscalls. Without them, macOS can fail Codex startup with a bare
    // `Operation not permitted`; revisit this if future macOS versions move
    // or rename the private Sandbox/67 operation.
    out.push_str("(allow system-mac-syscall (mac-policy-name \"vnguard\"))\n");
    out.push_str(
        "(allow system-mac-syscall (require-all (mac-policy-name \"Sandbox\") (mac-syscall-number 67)))\n",
    );
    out.push_str("(allow network*)\n");
    out.push_str("(allow sysctl*)\n");
    out.push_str("(allow iokit*)\n");

    out.push_str("(allow file-write* (subpath \"/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/tmp\"))\n");
    out.push_str("(allow file-write* (subpath \"/private/var/folders\"))\n");
    out.push_str("(allow file-write* (subpath \"/dev\"))\n");
    if let Some(home) = non_empty_env_path(home) {
        let home = home.display().to_string();
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
    if let Some(codex_dir) = codex_state_dir(home, codex_home) {
        // Codex initializes local state before it reads Orbit's envelope. Keep
        // this provider allowance narrow instead of granting broad HOME writes.
        out.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            sbpl_escape(&codex_dir.display().to_string())
        ));
    }

    for rule in &rules.modify {
        if let Some(deny_path) = rule.strip_prefix('!') {
            out.push_str(&format!(
                "(deny file-write* {})\n",
                sbpl_filter_for_deny_rule(deny_path)
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
            out.push_str(&format!(
                "(deny file-read* {})\n",
                sbpl_filter_for_deny_rule(deny_path)
            ));
        }
    }

    Ok(out)
}

fn codex_state_dir(home: Option<&OsStr>, codex_home: Option<&OsStr>) -> Option<PathBuf> {
    non_empty_env_path(codex_home)
        .or_else(|| non_empty_env_path(home).map(|path| path.join(".codex")))
}

fn non_empty_env_path(value: Option<&OsStr>) -> Option<PathBuf> {
    let value = value?;
    if value.to_string_lossy().is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
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

fn sbpl_filter_for_deny_rule(rule: &str) -> String {
    if deny_rule_can_use_subpath(rule) {
        let path = subpath_root(rule);
        format!("(subpath \"{}\")", sbpl_escape(&path))
    } else {
        let regex = glob_rule_to_regex(rule);
        format!("(regex \"{}\")", sbpl_escape(&regex))
    }
}

fn deny_rule_can_use_subpath(rule: &str) -> bool {
    let trimmed = rule.trim_end_matches('/');
    if !contains_glob(trimmed) {
        return true;
    }
    let Some(prefix) = trimmed.strip_suffix("/**") else {
        return false;
    };
    !contains_glob(prefix)
}

fn contains_glob(value: &str) -> bool {
    value.contains('*') || value.contains('?')
}

fn glob_rule_to_regex(rule: &str) -> String {
    let mut out = String::from("^");
    let chars: Vec<char> = rule.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                if chars.get(i + 2) == Some(&'/') {
                    out.push_str("(?:.*/)?");
                    i += 3;
                } else {
                    out.push_str(".*");
                    i += 2;
                }
            }
            '*' => {
                out.push_str("[^/]*");
                i += 1;
            }
            '?' => {
                out.push_str("[^/]");
                i += 1;
            }
            c => {
                push_regex_escaped(&mut out, c);
                i += 1;
            }
        }
    }
    out.push('$');
    out
}

fn push_regex_escaped(out: &mut String, c: char) {
    if matches!(
        c,
        '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\'
    ) {
        out.push('\\');
    }
    out.push(c);
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

    fn compile_with_env(
        resolved: &ResolvedFsProfile,
        home: Option<&str>,
        codex_home: Option<&str>,
    ) -> String {
        compile_macos_sandbox_profile_with_env(
            resolved,
            home.map(OsStr::new),
            codex_home.map(OsStr::new),
        )
        .expect("compile")
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
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_with_env(&resolved, Some("/Users/test"), None);
        assert!(
            text.contains("(allow file-write* (subpath \"/Users/test/.orbit\"))"),
            "missing ~/.orbit write allow: {text}"
        );
    }

    #[test]
    fn compile_grants_write_access_to_codex_home_when_set() {
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_with_env(
            &resolved,
            Some("/Users/test"),
            Some("/var/folders/test/codex-home"),
        );
        assert!(
            text.contains("(allow file-write* (subpath \"/var/folders/test/codex-home\"))"),
            "missing CODEX_HOME write allow: {text}"
        );
        assert!(
            !text.contains("(allow file-write* (subpath \"/Users/test/.codex\"))"),
            "CODEX_HOME should take precedence over HOME fallback: {text}"
        );
    }

    #[test]
    fn compile_grants_write_access_to_home_codex_when_codex_home_missing() {
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_with_env(&resolved, Some("/Users/test"), None);
        assert!(
            text.contains("(allow file-write* (subpath \"/Users/test/.codex\"))"),
            "missing HOME/.codex write allow: {text}"
        );
    }

    #[test]
    fn compile_allows_macos_sandbox_provenance_syscall() {
        let resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo/src"]);
        let text = compile_with_env(&resolved, Some("/Users/test"), None);
        assert!(
            text.contains("(allow system-mac-syscall (mac-policy-name \"vnguard\"))"),
            "missing vnguard mac syscall allow: {text}"
        );
        assert!(
            text.contains(
                "(allow system-mac-syscall (require-all (mac-policy-name \"Sandbox\") (mac-syscall-number 67)))"
            ),
            "missing Sandbox mac syscall allow: {text}"
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
    fn compile_uses_regex_for_non_subpath_negated_modify_glob() {
        let mut resolved = profile("default", &["/Users/test/repo"], &["/Users/test/repo"]);
        resolved
            .modify
            .push("!/Users/test/repo/**/*.env".to_string());
        let text = compile_macos_sandbox_profile(&resolved).expect("compile");
        assert!(
            text.contains(
                "(deny file-write* (regex \"^/Users/test/repo/(?:.*/)?[^/]*\\\\.env$\"))"
            ),
            "missing regex deny for env glob: {text}"
        );
        assert!(
            !text.contains("(deny file-write* (subpath \"/Users/test/repo\"))"),
            "env glob must not collapse to a repo-wide deny: {text}"
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
    #[test]
    fn compiled_profile_denies_env_glob_without_blocking_other_writes() {
        use std::process::Command;

        let home = std::env::var("HOME").expect("HOME set on macOS");
        let parent = std::path::PathBuf::from(home)
            .join(format!(".orbit-sandbox-test-{}", std::process::id()));
        std::fs::create_dir_all(&parent).expect("parent dir");
        let _cleanup = ScopeGuard(parent.clone());
        let dir = tempfile::Builder::new()
            .prefix("compile-env-")
            .tempdir_in(&parent)
            .expect("tempdir in parent");

        let resolved = ResolvedFsProfile {
            name: "default".to_string(),
            read: vec![dir.path().display().to_string()],
            modify: vec![
                dir.path().display().to_string(),
                format!("!{}/**/*.env", dir.path().display()),
            ],
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

        let allowed_target = dir.path().join("ok.txt");
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
            "env glob deny should not block non-env writes; status={allow_status:?}"
        );

        let env_target = dir.path().join("blocked.env");
        let deny_status = Command::new("sandbox-exec")
            .arg("-f")
            .arg(profile_file.path())
            .arg("/bin/sh")
            .arg("-c")
            .arg(format!("echo bad > {}", shell_escape(&env_target)))
            .status()
            .expect("run sandbox-exec");
        assert!(
            !deny_status.success(),
            "expected env glob write to fail; status={deny_status:?}"
        );
        assert!(
            !env_target.exists(),
            "env file should not exist: {env_target:?}"
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
