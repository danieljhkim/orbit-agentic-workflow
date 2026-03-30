use std::collections::HashMap;

use orbit_exec::{EnvironmentMode, ExecRequest, NoSandbox, StdinMode, run_process};
use orbit_types::{OrbitError, ReviewThread};
use serde_json::{Value, json};

use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};

const TIMEOUT_MS: u64 = 15_000;
type PrFilePatchMap = HashMap<String, Option<String>>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ThreadSyncMode {
    Inline { path: String, line: u64 },
    General,
}

pub(super) fn sync_batch_review_to_github<H: RuntimeHost + TaskHost + ?Sized>(
    host: &H,
    input: &Value,
) -> Result<Value, OrbitError> {
    let batch_id = input
        .get("run_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            OrbitError::InvalidInput(
                "sync_batch_review_to_github requires input.run_id".to_string(),
            )
        })?;

    let batch_tasks = host.list_tasks_filtered(None, None, None, Some(batch_id))?;
    let mut total: u64 = 0;

    for task in &batch_tasks {
        if task.pr_number.is_none() {
            continue;
        }
        if task.review_threads.is_empty() {
            continue;
        }
        total += sync_task_review_to_github(host, &task.id)?;
    }

    Ok(json!({ "synced_count": total }))
}

fn sync_task_review_to_github<H: RuntimeHost + TaskHost + ?Sized>(
    host: &H,
    task_id: &str,
) -> Result<u64, OrbitError> {
    let task = host.get_task(task_id)?;

    if task.pr_number.is_none() {
        return Ok(0);
    }

    if task.review_threads.is_empty() {
        return Ok(0);
    }

    let pr_number = task.pr_number.as_deref().unwrap();
    let repo_root = task
        .repo_root
        .as_deref()
        .or(task.workspace_path.as_deref())
        .ok_or_else(|| {
            OrbitError::InvalidInput(
                "sync_review_to_github requires task.repo_root or task.workspace_path".to_string(),
            )
        })?;

    let owner_repo = get_owner_repo(repo_root)?;
    let head_sha = get_pr_head_sha(repo_root, pr_number)?;
    // If patch metadata can't be resolved, fall back to general PR comments
    // instead of failing the entire review sync run.
    let pr_file_patches =
        load_pr_file_patches(repo_root, &owner_repo, pr_number).unwrap_or_default();

    let mut threads = task.review_threads.clone();
    let mut synced_count: u64 = 0;

    for thread in threads.iter_mut() {
        let thread_synced = sync_thread(
            repo_root,
            &owner_repo,
            pr_number,
            &head_sha,
            &pr_file_patches,
            thread,
        )?;
        synced_count += thread_synced;
    }

    if synced_count > 0 {
        host.apply_task_automation_update(
            task_id,
            TaskAutomationUpdate {
                review_threads: Some(threads),
                ..TaskAutomationUpdate::default()
            },
        )?;
    }

    Ok(synced_count)
}

fn sync_thread(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    head_sha: &str,
    pr_file_patches: &PrFilePatchMap,
    thread: &mut ReviewThread,
) -> Result<u64, OrbitError> {
    let mut synced: u64 = 0;
    let thread_path = thread.path.clone();
    let thread_line = thread.line;
    let sync_mode = sync_mode_for_thread(thread_path.as_deref(), thread_line, pr_file_patches);

    if thread.github_thread_id.is_none() && !thread.messages.is_empty() {
        let first_msg = &thread.messages[0];

        let github_id = match &sync_mode {
            ThreadSyncMode::Inline { path, line } => create_inline_review_comment(
                repo_root,
                owner_repo,
                pr_number,
                head_sha,
                path,
                *line,
                &first_msg.body,
            )?,
            ThreadSyncMode::General => create_general_comment(
                repo_root,
                pr_number,
                &render_general_comment_body(thread_path.as_deref(), thread_line, &first_msg.body),
            )?,
        };

        thread.github_thread_id = Some(github_id);
        thread.messages[0].github_comment_id = Some(github_id);
        synced += 1;
    }

    match &sync_mode {
        ThreadSyncMode::Inline { .. } => {
            if let Some(parent_id) = thread.github_thread_id {
                for msg in thread.messages.iter_mut().skip(1) {
                    if msg.github_comment_id.is_some() {
                        continue;
                    }
                    let reply_id = create_reply_comment(
                        repo_root, owner_repo, pr_number, parent_id, &msg.body,
                    )?;
                    msg.github_comment_id = Some(reply_id);
                    synced += 1;
                }
            }
        }
        ThreadSyncMode::General => {
            for msg in thread.messages.iter_mut().skip(1) {
                if msg.github_comment_id.is_some() {
                    continue;
                }
                let comment_id = create_general_comment(
                    repo_root,
                    pr_number,
                    &render_general_comment_body(thread_path.as_deref(), thread_line, &msg.body),
                )?;
                msg.github_comment_id = Some(comment_id);
                synced += 1;
            }
        }
    }

    Ok(synced)
}

fn sync_mode_for_thread(
    path: Option<&str>,
    line: Option<u64>,
    pr_file_patches: &PrFilePatchMap,
) -> ThreadSyncMode {
    match (path, line) {
        (Some(path), Some(line))
            if pr_file_patches
                .get(path)
                .and_then(|patch| patch.as_deref())
                .is_some_and(|patch| patch_supports_right_side_line(patch, line)) =>
        {
            ThreadSyncMode::Inline {
                path: path.to_string(),
                line,
            }
        }
        _ => ThreadSyncMode::General,
    }
}

fn render_general_comment_body(path: Option<&str>, line: Option<u64>, body: &str) -> String {
    match (path, line) {
        (Some(path), Some(line)) => format!("On `{path}:{line}`:\n\n{body}"),
        _ => body.to_string(),
    }
}

fn get_owner_repo(repo_root: &str) -> Result<String, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "repo".to_string(),
                "view".to_string(),
                "--json".to_string(),
                "nameWithOwner".to_string(),
                "-q".to_string(),
                ".nameWithOwner".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to get repo owner/name: {}",
            result.stderr.trim()
        )));
    }

    Ok(result.stdout.trim().to_string())
}

fn get_pr_head_sha(repo_root: &str, pr_number: &str) -> Result<String, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "pr".to_string(),
                "view".to_string(),
                pr_number.to_string(),
                "--json".to_string(),
                "headRefOid".to_string(),
                "-q".to_string(),
                ".headRefOid".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to get PR head SHA: {}",
            result.stderr.trim()
        )));
    }

    Ok(result.stdout.trim().to_string())
}

fn load_pr_file_patches(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
) -> Result<PrFilePatchMap, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "api".to_string(),
                format!("repos/{owner_repo}/pulls/{pr_number}/files"),
                "--paginate".to_string(),
                "--slurp".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to fetch PR file patches: {}",
            result.stderr.trim()
        )));
    }

    parse_pr_file_patches(&result.stdout)
}

fn parse_pr_file_patches(stdout: &str) -> Result<PrFilePatchMap, OrbitError> {
    let payload: Value = serde_json::from_str(stdout.trim()).map_err(|error| {
        OrbitError::Execution(format!(
            "failed to parse gh api pull request files output: {error}"
        ))
    })?;

    let mut patches = HashMap::new();
    for item in flatten_paginated_items(payload, "pull request files")? {
        let Value::Object(file) = item else {
            return Err(OrbitError::Execution(
                "gh api pull request files returned non-object item".to_string(),
            ));
        };
        let filename = file
            .get("filename")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                OrbitError::Execution(
                    "gh api pull request files returned item without filename".to_string(),
                )
            })?;
        let patch = file.get("patch").and_then(Value::as_str).map(String::from);
        patches.insert(filename.to_string(), patch);
    }

    Ok(patches)
}

fn flatten_paginated_items(payload: Value, label: &str) -> Result<Vec<Value>, OrbitError> {
    match payload {
        Value::Array(items) => {
            let mut flattened = Vec::new();
            for item in items {
                match item {
                    Value::Array(page) => flattened.extend(page),
                    Value::Object(_) => flattened.push(item),
                    other => {
                        return Err(OrbitError::Execution(format!(
                            "gh api {label} returned unexpected item type: {}",
                            json_type_name(&other)
                        )));
                    }
                }
            }
            Ok(flattened)
        }
        other => Err(OrbitError::Execution(format!(
            "gh api {label} returned unexpected payload type: {}",
            json_type_name(&other)
        ))),
    }
}

fn patch_supports_right_side_line(patch: &str, target_line: u64) -> bool {
    if target_line == 0 {
        return false;
    }

    let mut current_new_line: Option<u64> = None;

    for line in patch.lines() {
        if let Some(start_line) = parse_hunk_new_start(line) {
            current_new_line = Some(start_line);
            continue;
        }

        let Some(new_line) = current_new_line.as_mut() else {
            continue;
        };

        match line.as_bytes().first().copied() {
            Some(b' ') | Some(b'+') => {
                if *new_line == target_line {
                    return true;
                }
                *new_line += 1;
            }
            Some(b'-') => {}
            _ => {}
        }
    }

    false
}

fn parse_hunk_new_start(line: &str) -> Option<u64> {
    if !line.starts_with("@@") {
        return None;
    }

    line.split_whitespace()
        .find(|segment| segment.starts_with('+'))
        .and_then(|segment| segment.trim_start_matches('+').split(',').next())
        .and_then(|start| start.parse::<u64>().ok())
}

fn create_inline_review_comment(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    commit_id: &str,
    path: &str,
    line: u64,
    body: &str,
) -> Result<u64, OrbitError> {
    let payload = json!({
        "body": body,
        "commit_id": commit_id,
        "path": path,
        "line": line,
        "side": "RIGHT",
    });

    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "api".to_string(),
                format!("repos/{owner_repo}/pulls/{pr_number}/comments"),
                "--method".to_string(),
                "POST".to_string(),
                "--input".to_string(),
                "-".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Bytes(payload.to_string().into_bytes()),
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create inline review comment: {}",
            result.stderr.trim()
        )));
    }

    parse_comment_id(&result.stdout)
}

fn create_general_comment(repo_root: &str, pr_number: &str, body: &str) -> Result<u64, OrbitError> {
    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "pr".to_string(),
                "comment".to_string(),
                pr_number.to_string(),
                "--body".to_string(),
                body.to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Null,
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create PR comment: {}",
            result.stderr.trim()
        )));
    }

    // gh pr comment outputs a URL like https://github.com/owner/repo/pull/1#issuecomment-123
    // but doesn't return structured JSON by default. Use the API instead.
    // Actually, let's use the API for general comments too for consistency.
    // Fall back: parse the URL for the comment ID, or return 0 if unparseable.
    // For general comments via `gh pr comment`, the output is a URL.
    // Extract comment ID from the URL fragment.
    let output = result.stdout.trim();
    if let Some(id_str) = output.rsplit("issuecomment-").next()
        && let Ok(id) = id_str.trim().parse::<u64>()
    {
        return Ok(id);
    }

    // If we can't parse the ID from the URL, return an error rather than silently losing it
    Err(OrbitError::Execution(format!(
        "could not parse comment ID from gh pr comment output: {output}"
    )))
}

fn create_reply_comment(
    repo_root: &str,
    owner_repo: &str,
    pr_number: &str,
    parent_comment_id: u64,
    body: &str,
) -> Result<u64, OrbitError> {
    let payload = json!({ "body": body });

    let result = run_process(
        &ExecRequest {
            program: "gh".to_string(),
            args: vec![
                "api".to_string(),
                format!(
                    "repos/{owner_repo}/pulls/{pr_number}/comments/{parent_comment_id}/replies"
                ),
                "--method".to_string(),
                "POST".to_string(),
                "--input".to_string(),
                "-".to_string(),
            ],
            current_dir: Some(repo_root.to_string()),
            timeout_ms: Some(TIMEOUT_MS),
            stdin_mode: StdinMode::Bytes(payload.to_string().into_bytes()),
            environment_mode: EnvironmentMode::Inherit,
            debug: false,
        },
        &NoSandbox,
    )?;

    if !result.success {
        return Err(OrbitError::Execution(format!(
            "failed to create reply comment: {}",
            result.stderr.trim()
        )));
    }

    parse_comment_id(&result.stdout)
}

fn parse_comment_id(json_output: &str) -> Result<u64, OrbitError> {
    let value: Value = serde_json::from_str(json_output.trim())
        .map_err(|e| OrbitError::Execution(format!("failed to parse GitHub API response: {e}")))?;

    value
        .get("id")
        .and_then(Value::as_u64)
        .ok_or_else(|| OrbitError::Execution("GitHub API response missing 'id' field".to_string()))
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use super::{
        ThreadSyncMode, parse_pr_file_patches, patch_supports_right_side_line,
        render_general_comment_body, sync_mode_for_thread,
    };

    #[test]
    fn patch_supports_right_side_line_matches_context_and_added_lines() {
        let patch = "\
@@ -6,4 +6,5 @@ use std::time::Duration;
 use orbit_types::{JobRunState, OrbitError, Task, TaskStatus};
 use serde_json::{Value, json};
 
-use super::git::{git_command_success, git_output, git_success, resolve_worktree_start_point};
+use super::git::{git_output, git_success, resolve_worktree_start_point};
+use crate::context::{RuntimeHost, TaskAutomationUpdate, TaskHost};";

        assert!(patch_supports_right_side_line(patch, 6));
        assert!(patch_supports_right_side_line(patch, 7));
        assert!(patch_supports_right_side_line(patch, 8));
        assert!(patch_supports_right_side_line(patch, 9));
        assert!(patch_supports_right_side_line(patch, 10));
    }

    #[test]
    fn patch_supports_right_side_line_rejects_lines_outside_the_diff() {
        let patch = "\
@@ -6,4 +6,4 @@ use std::time::Duration;
 use orbit_types::{JobRunState, OrbitError, Task, TaskStatus};
 use serde_json::{Value, json};
 
-use super::git::{git_command_success, git_output, git_success, resolve_worktree_start_point};
+use super::git::{git_output, git_success, resolve_worktree_start_point};";

        assert!(!patch_supports_right_side_line(patch, 391));
        assert!(!patch_supports_right_side_line(patch, 0));
    }

    #[test]
    fn sync_mode_falls_back_to_general_for_non_diff_line() {
        let mut patches = HashMap::new();
        patches.insert(
            "orbit-engine/src/executor/automation/parallel.rs".to_string(),
            Some(
                "\
@@ -6,4 +6,4 @@ use std::time::Duration;
 use orbit_types::{JobRunState, OrbitError, Task, TaskStatus};
 use serde_json::{Value, json};
 
-use super::git::{git_command_success, git_output, git_success, resolve_worktree_start_point};
+use super::git::{git_output, git_success, resolve_worktree_start_point};"
                    .to_string(),
            ),
        );

        assert_eq!(
            sync_mode_for_thread(
                Some("orbit-engine/src/executor/automation/parallel.rs"),
                Some(391),
                &patches,
            ),
            ThreadSyncMode::General
        );
    }

    #[test]
    fn render_general_comment_body_preserves_inline_location_context() {
        assert_eq!(
            render_general_comment_body(Some("src/lib.rs"), Some(42), "Needs attention"),
            "On `src/lib.rs:42`:\n\nNeeds attention"
        );
        assert_eq!(
            render_general_comment_body(None, None, "Summary"),
            "Summary".to_string()
        );
    }

    #[test]
    fn parse_pr_file_patches_handles_slurped_pages() {
        let payload = json!([
            [
                {
                    "filename": "src/lib.rs",
                    "patch": "@@ -1 +1 @@\n-old\n+new"
                }
            ],
            [
                {
                    "filename": "README.md"
                }
            ]
        ]);

        let parsed = parse_pr_file_patches(&payload.to_string()).expect("parse patches");

        assert_eq!(
            parsed.get("src/lib.rs").and_then(|value| value.as_deref()),
            Some("@@ -1 +1 @@\n-old\n+new")
        );
        assert!(parsed.get("README.md").is_some_and(|value| value.is_none()));
    }
}
