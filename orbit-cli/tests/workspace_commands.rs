use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

/// Returns a Command targeting the orbit binary, with HOME pointed at `home_dir`.
/// Does NOT set ORBIT_ROOT, allowing workspace resolution to work naturally.
fn orbit_ws(home_dir: &Path, cwd: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(cwd);
    cmd.env("HOME", home_dir);
    cmd.env("USERPROFILE", home_dir);
    cmd.env_remove("ORBIT_ROOT");
    cmd.env_remove("ORBIT_WORKSPACE");
    cmd
}

/// Standard orbit helper with ORBIT_ROOT set (for commands that need a runtime).
fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd.env("HOME", dir);
    cmd.env("USERPROFILE", dir);
    cmd.env("ORBIT_ROOT", dir.join(".orbit"));
    cmd
}

#[test]
fn workspace_init_creates_orbit_dir_and_registers() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("myproject");
    std::fs::create_dir_all(&project).unwrap();

    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "myproj"])
        .assert()
        .success()
        .stdout(predicate::str::contains("workspace 'myproj' initialized"));

    // .orbit dir created
    assert!(project.join(".orbit").is_dir());

    // registry contains the workspace
    let registry_path = home.path().join(".orbit").join("workspaces.json");
    assert!(registry_path.exists());
    let content = std::fs::read_to_string(&registry_path).unwrap();
    assert!(content.contains("myproj"));
    assert!(content.contains("ws_myproj"));
}

#[test]
fn workspace_init_defaults_name_to_dir_name() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("cool-project");
    std::fs::create_dir_all(&project).unwrap();

    orbit_ws(home.path(), &project)
        .args(["workspace", "init"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cool-project"));
}

#[test]
fn workspace_init_with_custom_base_branch() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--base-branch", "develop"])
        .assert()
        .success();

    let registry_path = home.path().join(".orbit").join("workspaces.json");
    let content = std::fs::read_to_string(&registry_path).unwrap();
    assert!(content.contains("develop"));
}

#[test]
fn workspace_list_shows_registered_workspaces() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    // Init a workspace first
    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "testws"])
        .assert()
        .success();

    // Now init orbit so we have a runtime, then list
    orbit_in(home.path()).args(["init"]).assert().success();

    orbit_ws(home.path(), home.path())
        .env("ORBIT_ROOT", home.path().join(".orbit"))
        .args(["workspace", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("testws"))
        .stdout(predicate::str::contains("active"));
}

#[test]
fn workspace_show_displays_current_workspace() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    // Init workspace (creates .orbit and registers)
    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "showtest"])
        .assert()
        .success();

    // Init orbit runtime data in the project's .orbit
    orbit_in(&project).args(["init"]).assert().success();

    // Show should find the workspace by matching orbit_dir
    orbit_ws(home.path(), &project)
        .env("ORBIT_ROOT", project.join(".orbit"))
        .args(["workspace", "show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("showtest"));
}

#[test]
fn workspace_remove_deregisters_without_deleting_orbit() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    // Init workspace
    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "removeme"])
        .assert()
        .success();

    // Init orbit runtime
    orbit_in(home.path()).args(["init"]).assert().success();

    // Remove
    orbit_ws(home.path(), home.path())
        .env("ORBIT_ROOT", home.path().join(".orbit"))
        .args(["workspace", "remove", "removeme"])
        .assert()
        .success()
        .stdout(predicate::str::contains("removed from registry"));

    // .orbit still exists
    assert!(project.join(".orbit").is_dir());

    // But not in registry
    let registry_path = home.path().join(".orbit").join("workspaces.json");
    let content = std::fs::read_to_string(&registry_path).unwrap();
    assert!(!content.contains("removeme"));
}

#[test]
fn workspace_remove_nonexistent_fails() {
    let home = tempfile::tempdir().unwrap();
    orbit_in(home.path()).args(["init"]).assert().success();

    orbit_ws(home.path(), home.path())
        .env("ORBIT_ROOT", home.path().join(".orbit"))
        .args(["workspace", "remove", "nope"])
        .assert()
        .failure();
}

#[test]
fn workspace_init_duplicate_name_fails() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "dup"])
        .assert()
        .success();

    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "dup"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn workspace_list_shows_invalid_for_missing_root() {
    let home = tempfile::tempdir().unwrap();
    let project = home.path().join("ephemeral");
    std::fs::create_dir_all(&project).unwrap();

    // Init workspace
    orbit_ws(home.path(), &project)
        .args(["workspace", "init", "--name", "ghost"])
        .assert()
        .success();

    // Delete the project root
    std::fs::remove_dir_all(&project).unwrap();

    // Init orbit runtime at home so list has a runtime
    orbit_in(home.path()).args(["init"]).assert().success();

    // List should show invalid
    orbit_ws(home.path(), home.path())
        .env("ORBIT_ROOT", home.path().join(".orbit"))
        .args(["workspace", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("invalid"));
}

#[test]
fn backward_compat_orbit_root_still_works() {
    let home = tempfile::tempdir().unwrap();
    let custom_root = home.path().join("custom-root");
    std::fs::create_dir_all(&custom_root).unwrap();

    // Init with ORBIT_ROOT override
    orbit_in(home.path())
        .env("ORBIT_ROOT", &custom_root)
        .args(["init"])
        .assert()
        .success();

    // Task add should work with ORBIT_ROOT
    orbit_in(home.path())
        .env("ORBIT_ROOT", &custom_root)
        .args(["task", "add", "--title", "test task"])
        .assert()
        .success();
}
