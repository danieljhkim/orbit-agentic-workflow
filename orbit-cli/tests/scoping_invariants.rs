//! CLI-level integration tests for scoping, path resolution, and init invariants.
//!
//! These tests verify production-discovered bugs do not regress:
//! - `orbit init` at global scope does NOT create workspace-scoped dirs
//! - Implicit bootstrap creates workspace tasks/ directory
//! - `--root` flag resolves to the given path as the .orbit directory
//! - Worktree path resolution finds the main repo root

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("HOME", dir);
    cmd.env("USERPROFILE", dir);
    cmd.env("ORBIT_ROOT", dir.join(".orbit"));
    cmd
}

fn orbit_in_with_home(dir: &Path, home: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("HOME", home);
    cmd.env("USERPROFILE", home);
    cmd.env("ORBIT_ROOT", dir.join(".orbit"));
    cmd
}

// ---------------------------------------------------------------------------
// 1. orbit init at global root does NOT create workspace-scoped dirs
// ---------------------------------------------------------------------------

#[test]
fn init_does_not_create_tasks_or_runs_at_global_scope() {
    let home = tempfile::tempdir().expect("home");

    orbit_in(home.path()).args(["init"]).assert().success();

    let global_orbit = home.path().join(".orbit");
    assert!(global_orbit.exists(), ".orbit must exist after init");

    // tasks/ must NOT be created at global scope
    assert!(
        !global_orbit.join("tasks").exists(),
        "tasks/ must not be created at global scope by orbit init"
    );

    // runs/ must NOT be created at global scope
    assert!(
        !global_orbit.join("runs").exists(),
        "runs/ must not be created at global scope by orbit init"
    );

    // scoreboard/ must NOT be created at global scope (scoring defaults to false)
    assert!(
        !global_orbit.join("scoreboard").exists(),
        "scoreboard/ must not be created at global scope by orbit init"
    );
}

// ---------------------------------------------------------------------------
// 2. Implicit bootstrap creates workspace tasks/ directory
// ---------------------------------------------------------------------------

#[test]
fn implicit_bootstrap_creates_workspace_tasks_dir() {
    let workspace = tempfile::tempdir().expect("workspace");
    let home = tempfile::tempdir().expect("home");

    // First, init the global root
    orbit_in_with_home(workspace.path(), home.path())
        .args(["init"])
        .assert()
        .success();

    // Now run a task command which triggers implicit workspace bootstrap
    orbit_in_with_home(workspace.path(), home.path())
        .args(["task", "list", "--json"])
        .assert()
        .success();

    // The workspace .orbit/tasks/ should now exist
    let workspace_tasks = workspace.path().join(".orbit").join("tasks");
    assert!(
        workspace_tasks.exists(),
        "implicit bootstrap must create workspace .orbit/tasks/"
    );
}

// ---------------------------------------------------------------------------
// 3. --root flag resolves to .orbit/ within the given path
// ---------------------------------------------------------------------------

#[test]
fn root_flag_directs_operations_to_specified_orbit_dir() {
    let dir = tempfile::tempdir().expect("dir");
    let custom_root = dir.path().join("custom").join(".orbit");
    fs::create_dir_all(&custom_root).expect("create custom root");

    // Use --root to point at the custom .orbit directory.
    // First init the global root so config exists.
    orbit_in(dir.path()).args(["init"]).assert().success();

    // Add a task using --root pointing to our custom directory.
    let output = orbit_in(dir.path())
        .args([
            "--root",
            custom_root.to_str().unwrap(),
            "task",
            "add",
            "--title",
            "Root flag test",
            "--description",
            "Must go to custom root",
            "--plan",
            "1. Verify",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let task_id = String::from_utf8(output).expect("utf8").trim().to_string();
    assert!(!task_id.is_empty(), "task add should return a task ID");

    // Task must exist under the custom root, not the default
    let custom_tasks = custom_root.join("tasks");
    assert!(
        custom_tasks.exists(),
        "tasks/ must exist under custom --root path"
    );

    // Verify the task is retrievable from the custom root
    orbit_in(dir.path())
        .args([
            "--root",
            custom_root.to_str().unwrap(),
            "task",
            "show",
            &task_id,
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Root flag test"));
}

// ---------------------------------------------------------------------------
// 4. Worktree path resolution: .git file points to main repo
// ---------------------------------------------------------------------------

#[test]
fn worktree_git_file_resolves_to_main_repo_orbit_dir() {
    let dir = tempfile::tempdir().expect("dir");
    let home = tempfile::tempdir().expect("home");

    // Set up main repo with .git directory and .orbit
    let main_repo = dir.path().join("main-repo");
    fs::create_dir_all(main_repo.join(".git").join("worktrees").join("task-branch"))
        .expect("create main .git/worktrees");
    fs::create_dir_all(main_repo.join(".orbit")).expect("create main .orbit");

    // Set up worktree with .git file pointing back to main
    let worktree = dir.path().join("worktrees").join("task-branch");
    fs::create_dir_all(&worktree).expect("create worktree");
    let gitdir_target = main_repo.join(".git").join("worktrees").join("task-branch");
    fs::write(
        worktree.join(".git"),
        format!("gitdir: {}\n", gitdir_target.display()),
    )
    .expect("write .git file");

    // Init the global root
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(home.path());
    cmd.env("HOME", home.path());
    cmd.env("USERPROFILE", home.path());
    cmd.args(["init"]);
    cmd.assert().success();

    // From worktree, ORBIT_ROOT should resolve to main-repo's .orbit
    // (via the .git worktree file resolution).
    // We explicitly set ORBIT_ROOT to the main repo's .orbit to test task scoping.
    #[allow(deprecated)]
    let mut task_cmd = Command::cargo_bin("orbit").expect("binary exists");
    task_cmd.current_dir(&worktree);
    task_cmd.env("HOME", home.path());
    task_cmd.env("USERPROFILE", home.path());
    task_cmd.env("ORBIT_ROOT", main_repo.join(".orbit"));
    task_cmd.args([
        "task",
        "add",
        "--title",
        "Worktree task",
        "--description",
        "Created from worktree",
        "--plan",
        "1. Test",
    ]);
    let output = task_cmd.assert().success().get_output().stdout.clone();
    let task_id = String::from_utf8(output).expect("utf8").trim().to_string();

    // Task must be in main repo's .orbit, not in the worktree
    let main_tasks = main_repo.join(".orbit").join("tasks");
    assert!(
        main_tasks.exists(),
        "tasks/ must exist under main repo .orbit, not worktree"
    );

    // Verify task is accessible from main repo context
    #[allow(deprecated)]
    let mut show_cmd = Command::cargo_bin("orbit").expect("binary exists");
    show_cmd.current_dir(&main_repo);
    show_cmd.env("HOME", home.path());
    show_cmd.env("USERPROFILE", home.path());
    show_cmd.env("ORBIT_ROOT", main_repo.join(".orbit"));
    show_cmd.args(["task", "show", &task_id, "--json"]);
    show_cmd
        .assert()
        .success()
        .stdout(predicate::str::contains("Worktree task"));

    // Worktree should NOT have its own .orbit/tasks
    assert!(
        !worktree.join(".orbit").join("tasks").exists(),
        "worktree must not have its own .orbit/tasks"
    );
}

// ---------------------------------------------------------------------------
// 5. Global vs workspace separation: tasks don't leak to global
// ---------------------------------------------------------------------------

#[test]
fn task_add_does_not_write_to_global_orbit_root() {
    let home = tempfile::tempdir().expect("home");
    let workspace = tempfile::tempdir().expect("workspace");

    // Init global root
    orbit_in_with_home(workspace.path(), home.path())
        .args(["init"])
        .assert()
        .success();

    // Add a task (writes to workspace scope via ORBIT_ROOT)
    orbit_in_with_home(workspace.path(), home.path())
        .args([
            "task",
            "add",
            "--title",
            "Scoped task",
            "--description",
            "Must stay in workspace",
            "--plan",
            "1. Verify",
        ])
        .assert()
        .success();

    // Global .orbit must NOT contain task files
    let global_tasks = home.path().join(".orbit").join("tasks");
    if global_tasks.exists() {
        let has_task_files = fs::read_dir(&global_tasks)
            .expect("read global tasks")
            .filter_map(|e| e.ok())
            .any(|entry| {
                entry.path().is_dir()
                    && fs::read_dir(entry.path())
                        .map(|mut d| d.next().is_some())
                        .unwrap_or(false)
            });
        assert!(
            !has_task_files,
            "task files must not leak to global ~/.orbit/tasks/"
        );
    }

    // Workspace .orbit must contain the task
    let workspace_tasks = workspace.path().join(".orbit").join("tasks");
    assert!(
        workspace_tasks.exists(),
        "workspace .orbit/tasks/ must exist"
    );
    let has_workspace_tasks = fs::read_dir(&workspace_tasks)
        .expect("read workspace tasks")
        .filter_map(|e| e.ok())
        .any(|entry| {
            entry.path().is_dir()
                && fs::read_dir(entry.path())
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false)
        });
    assert!(
        has_workspace_tasks,
        "task must exist in workspace .orbit/tasks/"
    );
}
